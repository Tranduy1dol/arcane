use crate::starkware_utils::commitment_tree::patricia_tree::patricia_tree::PatriciaTree;
use cairo_vm::Felt252;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Deserialize, Clone, Debug, Serialize, PartialEq)]
pub struct ContractState {
    pub contract_hash: Vec<u8>,
    pub storage_commitment_tree: PatriciaTree,
    pub nonce: Felt252,
}
