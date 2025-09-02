use std::sync::Arc;

use red_db_core::{
    db::Db,
    proto::{Command, Response},
};

use crate::{connection::base::BasicConnection, error::ClientResult};

pub struct FileConnection {
    db: Arc<Db>,
}

impl FileConnection {
    pub async fn new(db: Arc<Db>) -> Self {
        Self { db }
    }
}

impl BasicConnection for FileConnection {
    async fn execute(&mut self, command: Command) -> ClientResult<Response> {
        Ok(self.db.execute(command).await)
    }

    async fn is_healthy(&self) -> bool {
        true
    }
}
