use std::io;
use std::time;
use thiserror::Error;
/// Error type for KvStore
#[derive(Error, Debug)]
pub enum KvsError {
    /// IO Error type for KvStore
    #[error("io error")]
    Io(#[from] io::Error),
    /// Serde Error type for KvStore
    #[error("json serde error")]
    Serde(#[from] serde_json::Error),
    /// System time Error type for KvStore
    #[error("system time error")]
    SystemTimeError(#[from] time::SystemTimeError),
    /// Key not found Error type for KvStore
    #[error("key not found: {0})")]
    KeyNotFound(String),
    /// Unexpected command
    #[error("unexpected command: {0})")]
    UnexpectedCommand(String),
}
