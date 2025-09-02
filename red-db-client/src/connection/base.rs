use red_db_core::proto::{Command, Response};

use crate::error::ClientResult;

pub(crate) trait BasicConnection {
    async fn execute(&mut self, command: Command) -> ClientResult<Response>;
    async fn is_healthy(&self) -> bool;
}
