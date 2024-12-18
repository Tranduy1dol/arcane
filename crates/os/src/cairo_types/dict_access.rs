use cairo_vm::Felt252;
use cairo_type_derive::FieldOffsetGetters;

#[derive(FieldOffsetGetters)]
pub struct DictAccess {
    #[allow(unused)]
    pub key: Felt252,
    #[allow(unused)]
    pub prev_value: Felt252,
    #[allow(unused)]
    pub new_value: Felt252,
}