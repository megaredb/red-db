mod connection;
pub mod error;
mod pool;
#[cfg(test)]
mod tests;

use std::{net::SocketAddr, path::PathBuf};

use crate::{
    error::{ClientError, ClientResult},
    pool::PooledConnection,
};
use deadpool::managed::PoolError;
use pool::{ConnectionManager, ConnectionPool};
use red_db_core::proto::{Command, Response};

#[derive(Clone)]
pub struct Client {
    pool: ConnectionPool,
}

impl Client {
    pub async fn execute(&self, command: Command) -> ClientResult<Response> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| ClientError::Protocol(format!("Pool error: {e}")))?;

        conn.execute(command).await
    }

    pub async fn get_connection(&self) -> Result<PooledConnection, PoolError<ClientError>> {
        self.pool.get().await
    }

    pub fn status(&self) -> deadpool::managed::Status {
        self.pool.status()
    }

    pub async fn is_space_exists(&self, space_name: String) -> ClientResult<bool> {
        let command = Command::IsSpaceExists {
            space: space_name.to_string(),
        };

        match self.execute(command).await? {
            Response::Bool(value) => Ok(value),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    pub async fn space(&self, space_name: String) -> ClientResult<SpaceClient> {
        if !self.is_space_exists(space_name.clone()).await? {
            return Err(ClientError::Server(
                red_db_core::error::ServerError::SpaceNotFound(space_name),
            ));
        }

        Ok(SpaceClient {
            client: self,
            space_name,
        })
    }

    pub async fn create_space(&self, space_name: String) -> ClientResult<()> {
        let command = Command::CreateSpace {
            space: space_name.to_string(),
        };

        match self.execute(command).await? {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(ClientError::Server(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    pub async fn delete_space(&self, space_name: String) -> ClientResult<()> {
        let command = Command::DeleteSpace {
            space: space_name.to_string(),
        };

        match self.execute(command).await? {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(ClientError::Server(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}

pub struct ClientBuilder {
    max_pool_size: usize,
    server_addr: Option<SocketAddr>,
    aof_path: Option<PathBuf>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_server_addr<T: Into<SocketAddr>>(mut self, server_addr: T) -> Self {
        if self.aof_path.is_some() {
            panic!("You can't set server_addr and aof_path at the same time");
        }

        self.server_addr = Some(server_addr.into());
        self
    }

    pub fn with_max_pool_size(mut self, max_pool_size: usize) -> Self {
        self.max_pool_size = max_pool_size;
        self
    }

    pub fn with_aof_path(mut self, aof_path: PathBuf) -> Self {
        if self.server_addr.is_some() {
            panic!("You can't set server_addr and aof_path at the same time");
        }

        self.aof_path = Some(aof_path);
        self
    }

    pub async fn build(&self) -> ClientResult<Client> {
        if self.server_addr.is_none() && self.aof_path.is_none() {
            return Err(ClientError::NoConfig);
        }

        let manager: ConnectionManager = if let Some(aof_path) = &self.aof_path {
            ConnectionManager::with_file_path(aof_path.clone()).await
        } else {
            ConnectionManager::with_server_addr(self.server_addr.unwrap())
        };

        let pool = ConnectionPool::builder(manager)
            .max_size(self.max_pool_size)
            .build()
            .unwrap();

        Ok(Client { pool })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            max_pool_size: 1,
            server_addr: None,
            aof_path: None,
        }
    }
}

pub struct SpaceClient<'a> {
    client: &'a Client,
    space_name: String,
}

impl<'a> SpaceClient<'a> {
    pub async fn set(&self, key: &str, value: Vec<u8>) -> ClientResult<()> {
        let command = Command::Set {
            space: self.space_name.clone(),
            key: key.to_string(),
            value,
        };

        match self.client.execute(command).await? {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(ClientError::Server(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    pub async fn set_string(&self, key: &str, value: &str) -> ClientResult<()> {
        self.set(key, value.as_bytes().to_vec()).await
    }

    pub async fn get(&self, key: &str) -> ClientResult<Option<Vec<u8>>> {
        let command = Command::Get {
            space: self.space_name.clone(),
            key: key.to_string(),
        };

        match self.client.execute(command).await? {
            Response::Value(value) => Ok(value),
            Response::Error(e) => Err(ClientError::Server(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    pub async fn get_string(&self, key: &str) -> ClientResult<Option<String>> {
        match self.get(key).await? {
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| ClientError::Protocol(format!("Invalid UTF-8: {e}"))),
            None => Ok(None),
        }
    }

    pub async fn delete(&self, key: &str) -> ClientResult<()> {
        let command = Command::Delete {
            space: self.space_name.clone(),
            key: key.to_string(),
        };

        match self.client.execute(command).await? {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(ClientError::Server(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    pub async fn list_keys(&self) -> ClientResult<Vec<String>> {
        let command = Command::ListKeys {
            space: self.space_name.clone(),
        };

        match self.client.execute(command).await? {
            Response::Keys(keys) => Ok(keys),
            Response::Error(e) => Err(ClientError::Server(e)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
