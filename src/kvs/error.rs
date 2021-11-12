use std::backtrace::Backtrace;
use std::io;
use std::time;
use thiserror::Error;

/// Result wrapper for KvsError
pub type Result<T> = std::result::Result<T, KvsError>;
/// Error type for KvStore
#[derive(Error, Debug)]
pub enum KvsError {
    /// IO Error type for KvStore
    #[error("io error")]
    Io {
        #[from]
        source: io::Error,
        backtrace: Backtrace,
    },
    /// Serde Error type for KvStore
    #[error("json serde error")]
    Serde {
        #[from]
        source: serde_json::Error,
        backtrace: Backtrace,
    },
    /// System time Error type for KvStore
    #[error("system time error")]
    SystemTimeError {
        #[from]
        source: time::SystemTimeError,
        backtrace: Backtrace,
    },
    /// Key not found Error type for KvStore
    #[error("Key not found: {key})")]
    KeyNotFound { key: String, backtrace: Backtrace },
    /// Unexpected command
    #[error("unexpected command: {command})")]
    UnexpectedCommand {
        command: String,
        backtrace: Backtrace,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error), // source and Display delegate to anyhow::Error
}
