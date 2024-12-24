use crate::starkware_utils::commitment_tree::base_types::{Length, NodePath};
use crate::starkware_utils::commitment_tree::error::TreeError;
use crate::starkware_utils::commitment_tree::patricia_tree::patricia_tree::EMPTY_NODE_HASH;
use crate::storage::storage::HASH_BYTES;
use arcane_os_type::hash::Hash;
use num_bigint::BigUint;

#[derive(thiserror::Error, Debug)]
pub enum BinaryNodeError {
    #[allow(unused)]
    #[error("Left node hash is empty hash")]
    LeftNodeIsEmpty,
    #[allow(unused)]
    #[error("Right node hash is empty hash")]
    RightNodeIsEmpty,
}

pub struct BinaryNodeFact {
    pub left_node: Hash,
    pub right_node: Hash,
}

impl BinaryNodeFact {
    const PREIMAGE_LENGTH: usize = 2 * HASH_BYTES;

    #[allow(unused)]
    pub fn new(left_node: Hash, right_node: Hash) -> Result<Self, BinaryNodeError> {
        if left_node == EMPTY_NODE_HASH {
            return Err(BinaryNodeError::LeftNodeIsEmpty);
        }
        if right_node == EMPTY_NODE_HASH {
            return Err(BinaryNodeError::RightNodeIsEmpty);
        }

        Ok(Self {
            left_node,
            right_node,
        })
    }
}

pub struct EdgeNodeFact {
    pub bottom_node: Hash,
    pub edge_path: NodePath,
    pub edge_length: Length,
}

impl EdgeNodeFact {
    const PREIMAGE_LENGTH: usize = 2 * HASH_BYTES + 1;

    pub fn new(bottom_node: Hash, path: NodePath, length: Length) -> Result<Self, TreeError> {
        verify_path_value(&path, length)?;
        Ok(Self {
            bottom_node,
            edge_path: path,
            edge_length: length,
        })
    }

    pub fn new_unchecked(bottom_node: Hash, path: NodePath, length: Length) -> Self {
        debug_assert!(verify_path_value(&path, length).is_ok());
        Self {
            bottom_node,
            edge_path: path,
            edge_length: length,
        }
    }
}

pub fn verify_path_value(path: &NodePath, length: Length) -> Result<(), TreeError> {
    // TODO: NodePath probably needs to be defined as BigUint
    if path.0 >= (BigUint::from(1u64) << length.0) {
        return Err(TreeError::InvalidEdgePath(path.clone(), length));
    }
    Ok(())
}
