use cairo_type_derive::{CairoType, FieldOffsetGetters};
use cairo_vm::Felt252;

#[derive(CairoType, FieldOffsetGetters)]
pub struct NodeEdge {
    pub length: Felt252,
    pub path: Felt252,
    pub bottom: Felt252,
}
