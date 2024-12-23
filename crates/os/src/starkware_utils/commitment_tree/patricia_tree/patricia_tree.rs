use serde::{Deserialize, Serialize};
use crate::r#type::hash::Hash;
use crate::starkware_utils::commitment_tree::base_types::Height;

pub const EMPTY_NODE_HASH: [u8; 32] = [0; 32];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatriciaTree {
    pub root: Hash,
    pub height: Height,
}