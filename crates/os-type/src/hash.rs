use serde::{Deserialize, Serialize};

const EMPTY_HASH: [u8; 32] = [0; 32];

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

impl PartialEq<[u8; 32]> for Hash {
    fn eq(&self, other: &[u8; 32]) -> bool {
        &self.0 == other
    }
}

impl Hash {
    pub fn empty() -> Self {
        Self::from_bytes_be(EMPTY_HASH)
    }

    pub fn from_bytes_be(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_bytes_be_slice(bytes: &[u8]) -> Self {
        let mut array = [0u8; 32];
        let start = 32 - bytes.len();

        for (i, &byte) in bytes.iter().enumerate() {
            array[start + i] = byte;
        }

        Hash(array)
    }
}