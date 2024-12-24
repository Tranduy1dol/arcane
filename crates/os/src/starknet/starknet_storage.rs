use crate::starkware_utils::commitment_tree::base_types::TreeIndex;
use crate::starkware_utils::commitment_tree::error::TreeError;
use cairo_vm::Felt252;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

#[derive(thiserror::Error, Debug)]
pub enum CommitmentInfoError {
    #[error(transparent)]
    Tree(#[from] TreeError),

    #[error("Inconsistent commitment tree roots: expected {1}, got {0}")]
    UpdatedRootMismatch(BigUint, BigUint),
}

#[serde_as]
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct CommitmentInfo {
    #[serde_as(as = "Felt252Num")]
    pub previous_root: Felt252,
    #[serde_as(as = "Felt252Num")]
    pub updated_root: Felt252,
    pub tree_height: usize,
    #[serde_as(as = "HashMap<Felt252Str, Vec<Felt252Str>>")]
    pub commitment_facts: HashMap<Felt252, Vec<Felt252>>,
}

#[allow(async_fn_in_trait)]
pub trait PerContractStorage {
    async fn compute_commitment(&mut self) -> Result<CommitmentInfo, CommitmentInfoError>;
    async fn read(&mut self, key: TreeIndex) -> Option<Felt252>;
    fn write(&mut self, key: TreeIndex, value: Felt252);
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageLeaf {
    pub value: Felt252,
}

impl StorageLeaf {
    pub fn new(value: Felt252) -> Self {
        Self { value }
    }

    pub fn empty() -> Self {
        Self::new(Felt252::ZERO)
    }
}
