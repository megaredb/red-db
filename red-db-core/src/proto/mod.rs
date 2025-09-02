use bincode::{Decode, Encode};

use crate::error::ServerError;

#[derive(Encode, Decode, Debug, Clone)]
pub enum Command {
    Get {
        space: String,
        key: String,
    },
    Set {
        space: String,
        key: String,
        value: Vec<u8>,
    },
    Delete {
        space: String,
        key: String,
    },

    ListSpaces,
    ListKeys {
        space: String,
    },
    DeleteSpace {
        space: String,
    },
    CreateSpace {
        space: String,
    },
    IsSpaceExists {
        space: String,
    },
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum Response {
    Ok,
    Value(Option<Vec<u8>>),
    Keys(Vec<String>),
    Spaces(Vec<String>),
    Bool(bool),
    Error(ServerError),
}

impl From<ServerError> for Response {
    fn from(val: ServerError) -> Self {
        Response::Error(val)
    }
}

impl From<Vec<u8>> for Response {
    fn from(val: Vec<u8>) -> Self {
        Response::Value(Some(val))
    }
}

impl From<Option<Vec<u8>>> for Response {
    fn from(val: Option<Vec<u8>>) -> Self {
        Response::Value(val)
    }
}
