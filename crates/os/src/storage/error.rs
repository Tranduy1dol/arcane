use crate::starkware_utils::serializable::{DeserializeError, SerializeError};

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("Content not found in storage")]
    ContentNotFound,

    #[error(transparent)]
    Deserialize(#[from] DeserializeError),

    #[error(transparent)]
    Serialize(#[from] SerializeError),
}