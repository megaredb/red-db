pub mod error;
pub mod settings;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use red_db_core::{
    db::Db,
    proto::{Command, Response},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{debug, error, info, instrument};

use error::ConnectionError;
use settings::Settings;

pub async fn run_server(settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr: SocketAddr = format!("{}:{}", settings.host, settings.port).parse()?;

    info!("Starting red-db server on {}", bind_addr);

    let listener = TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {bind_addr}: {e}"));

    let db = Arc::new(Db::new(PathBuf::from(settings.aof_path)).await);

    info!("red-db server ready to accept connections");

    let shutdown_signal = tokio::signal::ctrl_c();

    tokio::select! {
        _ = accept_connections(listener, db) => {
            info!("Accept loop ended");
        }
        _ = shutdown_signal => {
            info!("Shutting down...");
        }
    }

    Ok(())
}

async fn accept_connections(listener: TcpListener, db: Arc<Db>) {
    loop {
        match listener.accept().await {
            Ok(conn) => {
                let db_clone = db.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(db_clone, conn).await {
                        debug!("Connection error: {:?}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

#[instrument(
    name = "connection",
    skip(db, conn),
    fields(
        client.addr = %conn.1,
    )
)]
async fn handle_connection(
    db: Arc<Db>,
    conn: (TcpStream, SocketAddr),
) -> Result<(), ConnectionError> {
    let (mut stream, _) = conn;
    stream.set_nodelay(true).expect("Failed to set nodelay");

    info!("New client connected");

    loop {
        let command = read_command(&mut stream).await?;

        if let Some(cmd) = command {
            let response = db.execute(cmd).await;

            write_response(&mut stream, response).await?;
        } else {
            break;
        }
    }

    info!("Connection closed");

    Ok(())
}

async fn read_command(stream: &mut TcpStream) -> Result<Option<Command>, ConnectionError> {
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            debug!("Client closed connection gracefully.");
            return Ok(None);
        }
        Err(e) => {
            debug!("Failed to read command length: {}", e);

            return Err(ConnectionError::Io(e));
        }
    }

    let len = u32::from_le_bytes(len_buf) as usize;

    if len > 1024 * 1024 {
        return Err(ConnectionError::CommandTooLarge);
    }

    let mut cmd_buf = vec![0u8; len];
    stream
        .read_exact(&mut cmd_buf)
        .await
        .map_err(ConnectionError::Io)?;

    bincode::decode_from_slice(&cmd_buf, bincode::config::standard())
        .map(|(cmd, _)| Some(cmd))
        .map_err(|e| ConnectionError::Protocol(format!("Decode error: {e}")))
}

async fn write_response(stream: &mut TcpStream, response: Response) -> Result<(), ConnectionError> {
    let data = bincode::encode_to_vec(&response, bincode::config::standard())
        .map_err(|e| ConnectionError::Protocol(format!("Encode error: {e}")))?;

    let len_bytes = (data.len() as u32).to_le_bytes();

    stream
        .write_all(&len_bytes)
        .await
        .map_err(ConnectionError::Io)?;
    stream.write_all(&data).await.map_err(ConnectionError::Io)?;

    Ok(())
}
