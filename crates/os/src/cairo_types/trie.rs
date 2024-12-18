use cairo_vm::Felt252;
use cairo_type_derive::{CairoType, FieldOffsetGetters};

#[derive(CairoType, FieldOffsetGetters)]
pub struct NodeEdge {
    pub length: Felt252,
    pub path: Felt252,
    pub bottom: Felt252,
}