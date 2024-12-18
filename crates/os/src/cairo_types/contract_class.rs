use cairo_vm::Felt252;
use cairo_type_derive::FieldOffsetGetters;

#[derive(FieldOffsetGetters)]
pub(crate) struct ContractClassComponentHashes {
    contract_class_version: Felt252,
    external_functions_hash: Felt252,
    l1_handlers_hash: Felt252,
    constructors_hash: Felt252,
    abi_hash: Felt252,
    sierra_program_hash: Felt252,
}