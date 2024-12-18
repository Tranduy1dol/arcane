use serde::{Deserialize, Serialize};
use crate::r#type::hash::Hash;
use crate::starkware_utils::base_types::Height;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatriciaTree {
    pub root: Hash,
    pub height: Height,
}