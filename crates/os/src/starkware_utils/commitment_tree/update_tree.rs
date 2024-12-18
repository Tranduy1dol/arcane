use std::collections::{HashMap, HashSet};
use crate::starkware_utils::commitment_tree::base_types::{Height, TreeIndex};
use crate::starkware_utils::commitment_tree::error::TreeError;

#[derive(Clone, Debug, PartialEq)]
pub enum TreeUpdate<LF>
where
    LF: Clone,
{
    Tuple(Box<UpdateTree<LF>>, Box<UpdateTree<LF>>),
    Leaf(LF),
}

pub type UpdateTree<LF> = Option<TreeUpdate<LF>>;

#[derive(Clone, Debug, PartialEq)]
pub enum DecodeNodeCase {
    Left,
    Right,
    Both,
}

#[derive(Clone, Debug)]
pub struct DecodedNode<'a, LF>
where
    LF: Clone,
{
    pub left_child: &'a Option<TreeUpdate<LF>>,
    pub right_child: &'a Option<TreeUpdate<LF>>,
    pub case: DecodeNodeCase,
}

pub fn decode_node<LF>(node: &TreeUpdate<LF>) -> Result<DecodedNode<LF>, TreeError>
where
    LF: Clone,
{
    match node {
        TreeUpdate::Tuple(left, right) => {
            let case = match (left.is_none(), right.is_none()) {
                (true, false) => DecodeNodeCase::Right,
                (false, true) => DecodeNodeCase::Left,
                (false, false) => DecodeNodeCase::Both,
                (true, true) => return Err(TreeError::IsEmpty),
            };
            Ok(DecodedNode { left_child: left.as_ref(), right_child: right.as_ref(), case })
        }
        TreeUpdate::Leaf(_) => Err(TreeError::IsLeaf),
    }
}

type Layer<LF> = HashMap<TreeIndex, TreeUpdate<LF>>;

pub fn build_update_tree<LF>(height: Height, modifications: Vec<(TreeIndex, LF)>) -> UpdateTree<LF>
where
    LF: Clone,
{
    // Bottom layer. This will prefer the last modification to an index.
    if modifications.is_empty() {
        return None;
    }

    // A layer is a dictionary from index in current merkle layer [0, 2**layer_height) to a tree.
    // A tree is either None, a leaf, or a pair of trees.
    let mut layer: Layer<LF> =
        modifications.into_iter().map(|(index, leaf_fact)| (index, TreeUpdate::Leaf(leaf_fact))).collect();

    for _ in 0..height.0 {
        let parents: HashSet<TreeIndex> = layer.keys().map(|key| key / 2u64).collect();
        let mut new_layer: Layer<LF> = Layer::new();

        for index in parents.into_iter() {
            let left_update = layer.get(&(&index * 2u64)).cloned();
            let right_update = layer.get(&(&index * 2u64 + 1u64)).cloned();

            new_layer.insert(index, TreeUpdate::Tuple(Box::new(left_update), Box::new(right_update)));
        }

        layer = new_layer;
    }

    // We reached layer_height=0, the top layer with only the root (with index 0).
    debug_assert!(layer.len() == 1);

    // unwrap() is safe here, 0 should always be in `layer` by construction
    Some(layer.remove(&0u64.into()).unwrap())
}
