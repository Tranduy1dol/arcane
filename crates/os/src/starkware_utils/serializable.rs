#[derive(thiserror::Error, Debug)]
pub enum SerializeError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Expected value to be at most {0} bytes once serialized")]
    ValueTooLong(usize),
}

#[derive(thiserror::Error, Debug)]
pub enum DeserializeError {
    // Right now we keep the raw serde error available for easier debugging.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Could not find a deserialization method that takes this number of bytes: {0}")]
    NoVariantWithLength(usize),

    #[error("Expected {0} bytes but got {1}")]
    LengthMismatch(usize, usize),
}

pub trait Serializable: Sized + SerializationPrefix {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError>;

    fn deserialize(data: &[u8]) -> Result<Self, DeserializeError>;
}

pub trait SerializationPrefix {
    fn class_name_prefix() -> Vec<u8> {
        let type_name = std::any::type_name::<Self>().to_string();
        let struct_name = type_name.split("::").last().unwrap().to_snake_case();
        struct_name.into_bytes()
    }

    fn prefix() -> Vec<u8> {
        Self::class_name_prefix()
    }
}
