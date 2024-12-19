use starknet_types_core::felt::Felt;

#[derive(Debug, Clone, PartialEq)]
pub struct ContractClassComponentHashes {
    contract_class_version: Felt,
    external_functions_hash: Felt,
    l1_handlers_hash: Felt,
    constructors_hash: Felt,
    abi_hash: Felt,
    sierra_program_hash: Felt,
}