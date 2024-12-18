use std::collections::HashMap;
use std::ops::{Add, Mul};
use cairo_vm::Felt252;
use cairo_vm::types::errors::math_errors::MathError;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use crate::starkware_utils::commitment_tree::base_types::{DescentMap, DescentPath, DescentStart, Height, Length, NodePath};
use crate::starkware_utils::commitment_tree::update_tree::{TreeUpdate, UpdateTree};

type Preimage = HashMap<Felt252, Vec<Felt252>>;
type Triplet = (Felt252, Felt252, Felt252);

#[derive(thiserror::Error, Debug)]
pub enum DescentError {
    #[error("Key not found in preimage: {0}")]
    PreimageNotFound(Felt252),

    #[error("Expected a branch")]
    IsNotBranch,

    #[error("The heights of the trees do not match")]
    TreeHeightMismatch,

    #[error(transparent)]
    Math(#[from] MathError),
}

#[allow(clippy::large_enum_variant)]
enum PreimageNode<'preimage> {
    Leaf,
    Branch { left: Option<PreimageNodeIterator<'preimage>>, right: Option<PreimageNodeIterator<'preimage>> },
}

struct PreimageNodeIterator<'preimage> {
    height: Height,
    preimage: &'preimage Preimage,
    node: Triplet,
    is_done: bool,
}

impl<'preimage> PreimageNodeIterator<'preimage> {
    fn new(height: Height, preimage: &'preimage Preimage, node: Triplet) -> Self {
        Self { height, preimage, node, is_done: false }
    }
}

impl<'preimage> Iterator for PreimageNodeIterator<'preimage> {
    type Item = Result<PreimageNode<'preimage>, DescentError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }
        self.is_done = true;

        // Check for children
        if self.height.0 == 0 {
            return Some(Ok(PreimageNode::Leaf));
        }
        let (left, right) = match get_children(self.preimage, &self.node) {
            Ok(children) => children,
            Err(e) => return Some(Err(e)),
        };
        let empty_node = empty_triplet();

        let left_child = if left == empty_node {
            None
        } else {
            Some(PreimageNodeIterator::new(self.height - 1, self.preimage, left))
        };
        let right_child = if right == empty_node {
            None
        } else {
            Some(PreimageNodeIterator::new(self.height - 1, self.preimage, right))
        };

        Some(Ok(PreimageNode::Branch { left: left_child, right: right_child }))
    }
}

fn empty_triplet() -> Triplet {
    (Felt252::ZERO, Felt252::ZERO, Felt252::ZERO)
}

fn canonic(preimage: &Preimage, node_hash: Felt252) -> Triplet {
    if let Some(back) = preimage.get(&node_hash) {
        if back.len() == 3 {
            return (back[0], back[1], back[2]);
        }
    }
    (Felt252::ZERO, Felt252::ZERO, node_hash)
}

fn get_children(preimage: &Preimage, node: &Triplet) -> Result<(Triplet, Triplet), DescentError> {
    let length = node.0;
    let word = node.1;
    let node_hash = node.2;

    if length == Felt252::ZERO {
        let (left, right) = if node_hash == Felt252::ZERO {
            (Felt252::ZERO, Felt252::ZERO)
        } else {
            let node_preimage = match preimage.get(&node_hash) {
                None => return Err(DescentError::PreimageNotFound(node_hash)),
                Some(value) => value,
            };
            // let node_preimage = preimage.get(&node_hash).ok_or(DescentError::PreimageNotFound(node_hash))?;
            (node_preimage[0], node_preimage[1])
        };

        return Ok((canonic(preimage, left), canonic(preimage, right)));
    }

    let length_u64 = length.to_u64().ok_or(MathError::Felt252ToU64Conversion(Box::new(length)))?;

    if word.to_biguint() >> (length_u64 - 1) == BigUint::from(0u64) {
        return Ok(((length - 1, word, node_hash), empty_triplet()));
    }

    Ok((empty_triplet(), (length - 1, word - Felt252::from(BigUint::from(1u64) << (length_u64 - 1)), node_hash)))
}

fn preimage_tree(height: Height, preimage: &Preimage, node: Triplet) -> PreimageNodeIterator {
    PreimageNodeIterator::new(height, preimage, node)
}

fn get_descents<LF>(
    mut height: Height,
    mut path: NodePath,
    mut update_tree: &UpdateTree<LF>,
    mut previous_tree: Option<PreimageNodeIterator>,
    mut new_tree: Option<PreimageNodeIterator>,
) -> Result<DescentMap, DescentError>
where
    LF: Clone,
{
    let mut descent_map = DescentMap::new();

    if update_tree.is_none() || height.0 == 0 {
        return Ok(descent_map);
    }

    // Find longest edge.
    let orig_height = height;
    let orig_path = path.clone();

    // Traverse all the trees simultaneously, as long as they all satisfy the descent condition,
    // to find the maximal descent subpath.
    // Compared to the Python implementation, we unroll the loop to avoid having to Box<dyn>
    // everything to emulate duck-typing.
    let (lefts, rights) = loop {
        let (update_left, update_right) = match update_tree {
            None => return Err(DescentError::TreeHeightMismatch),

            Some(TreeUpdate::Leaf(_)) => {
                return Err(DescentError::IsNotBranch);
            }
            Some(TreeUpdate::Tuple(left, right)) => (left.as_ref(), right.as_ref()),
        };

        let (previous_left, previous_right) = match previous_tree {
            None => (None, None),
            Some(mut iter) => match iter.next().ok_or(DescentError::TreeHeightMismatch)?? {
                PreimageNode::Leaf => {
                    return Err(DescentError::IsNotBranch);
                }
                PreimageNode::Branch { left, right } => (left, right),
            },
        };

        let (new_left, new_right) = match new_tree {
            None => (None, None),
            Some(mut iter) => match iter.next().ok_or(DescentError::TreeHeightMismatch)?? {
                PreimageNode::Leaf => {
                    return Err(DescentError::IsNotBranch);
                }
                PreimageNode::Branch { left, right } => (left, right),
            },
        };

        // Note: we decrement height in each branch to avoid having to clone the nodes.
        // This results in a bit of (ugly) duplication.
        if update_left.is_none() && previous_left.is_none() && new_left.is_none() {
            path = NodePath(path.0 * 2u64 + 1u64);
            height = Height(height.0 - 1);
            if height.0 == 0 {
                break ((update_left, previous_left, new_left), (update_right, previous_right, new_right));
            }

            update_tree = update_right;
            previous_tree = previous_right;
            new_tree = new_right;
        } else if update_right.is_none() && previous_right.is_none() && new_right.is_none() {
            path = NodePath(path.0 * 2u64);
            height = Height(height.0 - 1);
            if height.0 == 0 {
                break ((update_left, previous_left, new_left), (update_right, previous_right, new_right));
            }

            update_tree = update_left;
            previous_tree = previous_left;
            new_tree = new_left;
        } else {
            break ((update_left, previous_left, new_left), (update_right, previous_right, new_right));
        }
    };

    let length = orig_height.0 - height.0;
    // length <= 1 is not a descent.
    if length > 1 {
        descent_map.insert(
            DescentStart(orig_height, orig_path),
            DescentPath(Length(length), NodePath(path.0.clone() % (BigUint::from(1u64) << length))),
        );
    }

    if height.0 > 0 {
        let next_height = Height(height.0 - 1);
        descent_map.extend(get_descents(next_height, NodePath(path.0.clone().mul(2u64)), lefts.0, lefts.1, lefts.2)?);
        descent_map.extend(get_descents(
            next_height,
            NodePath(path.0.mul(2u64).add(1u64)),
            rights.0,
            rights.1,
            rights.2,
        )?);
    }

    Ok(descent_map)
}

pub fn patricia_guess_descents<LF>(
    height: Height,
    node: UpdateTree<LF>,
    preimage: &Preimage,
    prev_root: BigUint,
    new_root: BigUint,
) -> Result<DescentMap, DescentError>
where
    LF: Clone,
{
    let node_prev = preimage_tree(height, preimage, canonic(preimage, Felt252::from(prev_root)));
    let node_new = preimage_tree(height, preimage, canonic(preimage, Felt252::from(new_root)));

    get_descents::<LF>(height, NodePath(BigUint::from(0u64)), &node, Some(node_prev), Some(node_new))
}
