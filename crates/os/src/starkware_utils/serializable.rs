#[derive(thiserror::Error, Debug)]
pub enum SerializeError {
    // Right now we keep the raw serde error available for easier debugging.
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