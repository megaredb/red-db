use std::{path::PathBuf, sync::Arc};

use arc_swap::ArcSwap;
use rpds::HashTrieMapSync;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};
use tracing::{debug, error};

use crate::{
    error::ServerError,
    proto::{Command, Response},
    utils::HashedKey,
};

type SpaceData = HashTrieMapSync<HashedKey, Vec<u8>>;
type Store = HashTrieMapSync<String, SpaceData>;

#[derive(Clone)]
pub struct Db {
    data: Arc<ArcSwap<Store>>,
    aof_sender: mpsc::Sender<Command>,
}

impl Db {
    pub async fn new(aof_path: PathBuf) -> Self {
        let (aof_sender, aof_receiver) = mpsc::channel(1024);

        let initial_store = Self::restore_from_aof(&aof_path).await.unwrap_or_else(|e| {
            error!("Failed to restore from AOF: {}, starting fresh", e);
            Store::new_sync()
        });

        tokio::spawn(aof_writer_task(aof_receiver, aof_path));

        Self {
            data: Arc::new(ArcSwap::from(Arc::new(initial_store))),
            aof_sender,
        }
    }

    async fn restore_from_aof(aof_path: &PathBuf) -> Result<Store, ServerError> {
        if !aof_path.exists() {
            return Ok(Store::new_sync());
        }

        let mut file = fs::File::open(aof_path).await.map_err(|e| {
            error!("Failed to open AOF file: {}", e);
            ServerError::AofReadFailed
        })?;

        let mut store = Store::new_sync();

        loop {
            let mut len_bytes = [0u8; 4];
            match file.read_exact(&mut len_bytes).await {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    error!("Failed to read command from AOF: {}", e);
                    return Err(ServerError::AofReadFailed);
                }
            }

            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut command_bytes = vec![0u8; len];
            file.read_exact(&mut command_bytes).await.map_err(|e| {
                error!("Failed to read command from AOF: {}", e);
                ServerError::AofReadFailed
            })?;

            if let Ok((command, _)) =
                bincode::decode_from_slice(&command_bytes, bincode::config::standard())
            {
                store = Self::apply_command_to_store(store, &command);
            }
        }

        Ok(store)
    }

    fn apply_command_to_store(store: Store, command: &Command) -> Store {
        match command {
            Command::Set { space, key, value } => {
                let hashed_key = HashedKey::new(key.clone());
                let space_data = store
                    .get(space)
                    .cloned()
                    .unwrap_or_else(SpaceData::new_sync);
                let updated_space = space_data.insert(hashed_key, value.clone());
                store.insert(space.clone(), updated_space)
            }
            Command::Delete { space, key } => {
                let hashed_key = HashedKey::new(key.clone());
                if let Some(space_data) = store.get(space) {
                    let updated_space = space_data.remove(&hashed_key);
                    store.insert(space.clone(), updated_space)
                } else {
                    store
                }
            }
            Command::CreateSpace { space } => store.insert(space.clone(), SpaceData::new_sync()),
            Command::DeleteSpace { space } => store.remove(space),
            _ => store,
        }
    }

    fn validate_command(command: &Command) -> Result<(), ServerError> {
        match command {
            Command::Set { key, value, .. } => {
                if key.is_empty() {
                    return Err(ServerError::InvalidKey("Key cannot be empty".to_string()));
                }
                if value.len() > 1024 * 1024 {
                    return Err(ServerError::ValueTooLarge);
                }
            }
            Command::CreateSpace { space } => {
                if space.is_empty() || space.len() > 255 {
                    return Err(ServerError::InvalidSpaceName);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn execute(&self, command: Command) -> Response {
        match command {
            Command::Get { space, key } => {
                let hashed_key = HashedKey::new(key.clone());
                let db_snapshot = self.data.load();

                if let Some(space_data) = db_snapshot.get(&space) {
                    Response::Value(space_data.get(&hashed_key).cloned())
                } else {
                    Response::Error(ServerError::SpaceNotFound(space))
                }
            }
            Command::ListKeys { space } => {
                let db_snapshot = self.data.load();

                if let Some(space_data) = db_snapshot.get(&space) {
                    let keys = space_data.keys().map(|k| k.key.clone()).collect();
                    Response::Keys(keys)
                } else {
                    Response::Error(ServerError::SpaceNotFound(space))
                }
            }
            Command::ListSpaces => {
                let db_snapshot = self.data.load();

                let spaces = db_snapshot.keys().cloned().collect();
                Response::Spaces(spaces)
            }
            Command::IsSpaceExists { space } => {
                let db_snapshot = self.data.load();
                Response::Bool(db_snapshot.contains_key(&space))
            }
            _ => self.handle_write(command).await,
        }
    }

    async fn handle_write(&self, command: Command) -> Response {
        if let Err(err) = Self::validate_command(&command) {
            return Response::Error(err);
        }

        if self.aof_sender.send(command.clone()).await.is_err() {
            return Response::Error(ServerError::AofWriteFailed);
        }

        debug!("Received command: {:#?}", command);

        loop {
            let current_data_ptr = self.data.load();
            let mut new_data = (**current_data_ptr).clone();

            let result = match &command {
                Command::Set { space, key, value } => {
                    let hashed_key = HashedKey::new(key.clone());

                    let space_data = new_data
                        .get(space)
                        .cloned()
                        .unwrap_or_else(SpaceData::new_sync);

                    let updated_space_data = space_data.insert(hashed_key, value.clone());

                    new_data = new_data.insert(space.clone(), updated_space_data);
                    Ok(())
                }
                Command::Delete { space, key } => {
                    let hashed_key = HashedKey::new(key.clone());
                    match new_data.get(space) {
                        Some(space_data) => {
                            let updated_space_data = space_data.remove(&hashed_key);
                            new_data = new_data.insert(space.clone(), updated_space_data);
                            Ok(())
                        }
                        None => return Response::Error(ServerError::SpaceNotFound(space.clone())),
                    }
                }
                Command::DeleteSpace { space } => {
                    if !new_data.contains_key(space) {
                        return Response::Error(ServerError::SpaceNotFound(space.clone()));
                    }
                    new_data = new_data.remove(space);
                    Ok(())
                }
                Command::CreateSpace { space } => {
                    if new_data.contains_key(space) {
                        return Response::Error(ServerError::SpaceAlreadyExists(space.clone()));
                    }
                    new_data = new_data.insert(space.clone(), SpaceData::new_sync());
                    Ok(())
                }
                _ => unreachable!(),
            };

            if let Err(err) = result {
                return Response::Error(err);
            }

            if Arc::ptr_eq(
                &current_data_ptr,
                &self
                    .data
                    .compare_and_swap(&current_data_ptr, Arc::new(new_data)),
            ) {
                break;
            }
        }

        debug!("Applied command: {:#?}", command);

        Response::Ok
    }
}

async fn aof_writer_task(mut receiver: mpsc::Receiver<Command>, aof_path: PathBuf) {
    let mut file = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(aof_path)
        .await
    {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to open AOF file: {}", e);
            return;
        }
    };

    while let Some(command) = receiver.recv().await {
        if let Ok(serialized) = bincode::encode_to_vec(&command, bincode::config::standard()) {
            let len = serialized.len() as u32;
            if file.write_all(&len.to_le_bytes()).await.is_err()
                || file.write_all(&serialized).await.is_err()
                || file.flush().await.is_err()
            {
                error!("Failed to write command to AOF");
            }
        }
    }
}
