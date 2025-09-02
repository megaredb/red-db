use std::{net::SocketAddr, sync::Arc};

use red_db_core::{
    db::Db,
    proto::{Command, Response},
};

use crate::{
    connection::{base::BasicConnection, file::FileConnection, tcp::TcpConnection},
    error::ClientResult,
};

pub mod base;
pub mod file;
pub mod tcp;

enum ConnectionImpl {
    Tcp(TcpConnection),
    File(FileConnection),
}

pub struct Connection {
    connection_impl: ConnectionImpl,
}

impl Connection {
    pub async fn remote_connect(url: SocketAddr) -> ClientResult<Self> {
        Ok(Connection {
            connection_impl: ConnectionImpl::Tcp(TcpConnection::connect(url).await?),
        })
    }

    pub async fn use_db(db: Arc<Db>) -> Self {
        Connection {
            connection_impl: ConnectionImpl::File(FileConnection::new(db).await),
        }
    }

    pub async fn execute(&mut self, command: Command) -> ClientResult<Response> {
        match &mut self.connection_impl {
            ConnectionImpl::Tcp(tcp_connection) => tcp_connection.execute(command).await,
            ConnectionImpl::File(file_connection) => file_connection.execute(command).await,
        }
    }

    pub async fn is_healthy(&self) -> bool {
        match &self.connection_impl {
            ConnectionImpl::Tcp(tcp_connection) => tcp_connection.is_healthy().await,
            ConnectionImpl::File(file_connection) => file_connection.is_healthy().await,
        }
    }
}
