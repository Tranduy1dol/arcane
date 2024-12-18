use std::ops::Deref;
use blockifier::execution::syscalls::hint_processor::SyscallExecutionError;
use cairo_vm::Felt252;
use num_bigint::BigUint;
use starknet_crypto::{poseidon_hash_many, FieldElement};
use crate::r#type::hash::Hash;
use crate::starkware_utils::commitment_tree::base_types::Length;

#[derive(Clone, Debug)]
pub enum BytecodeSegmentStructureImpl {
    SegmentedNode(BytecodeSegmentedNode),
    Leaf(BytecodeLeaf),
}

#[derive(Clone, Debug)]
pub struct BytecodeSegmentedNode {
    pub segments: Vec<BytecodeSegment>,
}

impl BytecodeSegmentedNode {
    fn add_bytecode_with_skipped_segments(&self, data: &mut Vec<Felt252>) {
        for segment in &self.segments {
            if segment.is_used {
                segment.inner_structure.add_bytecode_with_skipped_segments(data);
            } else {
                // -1 marks the start of an unused segment.
                data.push(Felt252::from(-1));
                for _ in 0..(segment.segment_length.0 - 1) {
                    data.push(Felt252::from(-2));
                }
            }
        }
    }

    pub fn hash(&self) -> Result<Hash, SyscallExecutionError> {
        let mut felts = Vec::new();

        // To compute the hash we'll need the segment length and the hash from the inner structure for each
        // segment. After calling poseidon hash function, we just add 1 to the result
        for segment in &self.segments {
            felts.push(Felt252::from(segment.segment_length.0));

            let inner_hash = segment.inner_structure.hash()?;
            felts.push(Felt252::from_bytes_be_slice(inner_hash.deref()).map_err(|_| {
                SyscallExecutionError::FromStr("conversion from Hash to FieldElement failed".into())
            })?);
        }

        let ret = poseidon_hash_many(&felts) + Felt252::from(1u8);
        Ok(Hash::from_bytes_be(ret.to_bytes_be()))
    }
}

/// Represents a child of BytecodeSegmentedNode.
#[derive(Clone, Debug)]
pub struct BytecodeSegment {
    /// The length of the segment.
    pub segment_length: Length,
    /// Should the segment (or part of it) be loaded to memory.
    /// In other words, is the segment used during the execution.
    /// Note that if is_used is False, the entire segment is not loaded to memory.
    /// If is_used is True, it is possible that part of the segment will be skipped (according
    /// to the "is_used" field of the child segments).
    pub is_used: bool,
    /// The inner structure of the segment.
    pub inner_structure: BytecodeSegmentStructureImpl,
}

/// Represents a leaf in the bytecode segment tree.
#[derive(Clone, Debug)]
pub struct BytecodeLeaf {
    pub data: Vec<BigUint>,
}

impl BytecodeLeaf {
    fn add_bytecode_with_skipped_segments(&self, data: &mut Vec<Felt252>) {
        data.extend(self.data.iter().map(Felt252::from))
    }

    pub fn hash(&self) -> Result<Hash, SyscallExecutionError> {
        let vec_field_elements: Result<Vec<_>, _> =
            self.data.iter().map(|value| FieldElement::from_byte_slice_be(&value.to_bytes_be())).collect();

        let hash = match vec_field_elements {
            Ok(elements) => Hash::from_bytes_be(poseidon_hash_many(&elements).to_bytes_be()),
            Err(_) => {
                return Err(SyscallExecutionError::FromStr("Invalid bytecode segment leaf".into()));
            }
        };

        Ok(hash)
    }
}

impl BytecodeSegmentStructureImpl {
    /// Returns the bytecode of the node.
    /// Skipped segments are replaced with [-1, -2, -2, -2, ...].
    pub fn bytecode_with_skipped_segments(&self) -> Vec<Felt252> {
        let mut res = vec![];
        self.add_bytecode_with_skipped_segments(&mut res);

        res
    }

    fn add_bytecode_with_skipped_segments(&self, data: &mut Vec<Felt252>) {
        match self {
            BytecodeSegmentStructureImpl::SegmentedNode(node) => node.add_bytecode_with_skipped_segments(data),
            BytecodeSegmentStructureImpl::Leaf(leaf) => leaf.add_bytecode_with_skipped_segments(data),
        }
    }

    pub fn hash(&self) -> Result<Hash, SyscallExecutionError> {
        let ret = match self {
            BytecodeSegmentStructureImpl::SegmentedNode(node) => node.hash(),
            BytecodeSegmentStructureImpl::Leaf(leaf) => leaf.hash(),
        }?;

        Ok(ret)
    }
}