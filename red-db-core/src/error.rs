use bincode::{Decode, Encode};
use thiserror::Error;

#[derive(Encode, Decode, Debug, Clone, Error)]
pub enum ServerError {
    #[error("Space '{0}' not found")]
    SpaceNotFound(String),
    #[error("Key '{0}' not found in space '{1}'")]
    KeyNotFound(String, String),
    #[error("Space '{0}' already exists")]
    SpaceAlreadyExists(String),
    #[error("AOF write failed")]
    AofWriteFailed,
    #[error("AOF read failed")]
    AofReadFailed,
    #[error("Invalid key '{0}'")]
    InvalidKey(String),
    #[error("Invalid space name")]
    InvalidSpaceName,
    #[error("Value too large")]
    ValueTooLarge,
}
