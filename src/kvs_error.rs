use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, KvStoreError>;

#[derive(Error, Debug)]
pub enum KvStoreError {
    #[error("Failed to read/write")]
    IoError(#[from] io::Error),
    #[error("Failed to serialize")]
    SerdeSerError(#[from] serde_json::Error),
    #[error("No path")]
    No,
    #[error("Key not found")]
    KeyNotFound,
    #[error("Invalid log file command")]
    InvalidLogFileCommand,
    #[error("Invalid file")]
    InvalidFile,
    #[error("Failed to encode/decode")]
    BincodeError(#[from] bincode::Error),
}
