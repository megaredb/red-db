use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Connection error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Command too large")]
    CommandTooLarge,
}
