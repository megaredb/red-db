use std::net::SocketAddr;

use red_db_core::proto::{Command, Response};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::debug;

use crate::{
    connection::base::BasicConnection,
    error::{ClientError, ClientResult},
};

#[derive(Debug)]
pub struct TcpConnection {
    stream: TcpStream,
}

impl TcpConnection {
    pub async fn connect(to: SocketAddr) -> ClientResult<Self> {
        let stream = TcpStream::connect(to).await.map_err(ClientError::Io)?;

        stream.set_nodelay(true).expect("Failed to set nodelay");

        Ok(TcpConnection { stream })
    }

    async fn send_command(&mut self, command: &Command) -> ClientResult<()> {
        let data = bincode::encode_to_vec(command, bincode::config::standard())
            .map_err(|e| ClientError::Protocol(format!("Encode error: {e}")))?;

        let len_bytes = (data.len() as u32).to_le_bytes();

        self.stream
            .write_all(&len_bytes)
            .await
            .map_err(ClientError::Io)?;
        debug!("Sent command length: {}", data.len());
        self.stream
            .write_all(&data)
            .await
            .map_err(ClientError::Io)?;
        debug!("Sent command");

        Ok(())
    }

    async fn receive_response(&mut self) -> ClientResult<Response> {
        let mut len_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(ClientError::Io)?;

        let len = u32::from_le_bytes(len_bytes) as usize;

        if len > 16 * 1024 * 1024 {
            return Err(ClientError::Protocol("Response too large".to_string()));
        }

        let mut response_buf = vec![0u8; len];
        self.stream
            .read_exact(&mut response_buf)
            .await
            .map_err(ClientError::Io)?;

        let (response, _) = bincode::decode_from_slice(&response_buf, bincode::config::standard())
            .map_err(|e| ClientError::Protocol(format!("Decode error: {e}")))?;

        Ok(response)
    }
}

impl BasicConnection for TcpConnection {
    async fn execute(&mut self, command: Command) -> ClientResult<Response> {
        self.send_command(&command).await?;
        self.receive_response().await
    }

    // TODO: Improve health check.
    async fn is_healthy(&self) -> bool {
        let mut buf = [0u8; 0];
        matches!(self.stream.try_read(&mut buf), Ok(0) | Err(_))
    }
}
