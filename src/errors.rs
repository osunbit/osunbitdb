use thiserror::Error;
use tikv_client::Error as TiKVError;
use tokio::task::JoinError;
use bincode;

#[derive(Debug, Error)]
pub enum OsunbitDBError {
    #[error("TiKV client error: {0}")]
    TiKV(#[from] TiKVError),

    #[error("Serialization error: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),

    #[error("Join error: {0}")]
    Join(#[from] JoinError),

    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Invalid update: {0}")]
    InvalidUpdate(String),
}
