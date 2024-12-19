use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hash([u8; 32]);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct GenericClassHash(Hash);

impl GenericClassHash {
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }

    pub fn from_bytes_be(bytes: [u8; 32]) -> Self {
        Self(Hash(bytes))
    }
}