use red_db_core::error::ServerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Connection error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Server error: {0}")]
    Server(#[from] ServerError),
    #[error("Unexpected response from server")]
    UnexpectedResponse,
    #[error("Connection error pool: {0}")]
    Pool(String),
    #[error("No configuration")]
    NoConfig,
}

pub type ClientResult<T> = std::result::Result<T, ClientError>;
