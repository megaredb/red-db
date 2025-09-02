use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use deadpool::managed::{Manager, Object, Pool, RecycleError, RecycleResult};
use red_db_core::db::Db;

use crate::{connection::Connection, error::ClientError};

enum ConnectionUrl {
    Tcp(SocketAddr),
    File(Arc<Db>),
}

pub struct ConnectionManager {
    connection_url: ConnectionUrl,
}

impl Manager for ConnectionManager {
    type Type = Connection;
    type Error = ClientError;

    async fn create(&self) -> Result<Connection, Self::Error> {
        match &self.connection_url {
            ConnectionUrl::Tcp(addr) => Connection::remote_connect(*addr).await,
            ConnectionUrl::File(db) => Ok(Connection::use_db(Arc::clone(db)).await),
        }
    }

    async fn recycle(
        &self,
        conn: &mut Connection,
        _: &deadpool::managed::Metrics,
    ) -> RecycleResult<Self::Error> {
        if conn.is_healthy().await {
            Ok(())
        } else {
            Err(RecycleError::Backend(ClientError::Protocol(
                "Connection is unhealthy".to_string(),
            )))
        }
    }

    fn detach(&self, _obj: &mut Self::Type) {}
}

impl ConnectionManager {
    pub fn with_server_addr(server_addr: SocketAddr) -> Self {
        Self {
            connection_url: ConnectionUrl::Tcp(server_addr),
        }
    }

    pub async fn with_file_path(file_path: PathBuf) -> Self {
        let db = Arc::new(Db::new(file_path).await);

        Self {
            connection_url: ConnectionUrl::File(db),
        }
    }
}

pub type ConnectionPool = Pool<ConnectionManager>;
pub type PooledConnection = Object<ConnectionManager>;
