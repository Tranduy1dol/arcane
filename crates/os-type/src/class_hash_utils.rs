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

impl ContractClassComponentHashes {
    pub fn to_vec(self) -> Vec<Felt> {
        vec![
            self.contract_class_version,
            self.external_functions_hash,
            self.l1_handlers_hash,
            self.constructors_hash,
            self.abi_hash,
            self.sierra_program_hash,
        ]
    }
}