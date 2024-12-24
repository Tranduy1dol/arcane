use std::collections::HashMap;
use arcane_os_type::deprecated_compiled_class::GenericDeprecatedCompiledClass;
use arcane_os_type::casm_contract_class::GenericCasmContractClass;
use cairo_vm::Felt252;
use serde::{Deserialize, Serialize};
use starknet_types_core::felt::Felt;
use crate::config::StarknetGeneralConfig;
use crate::io::InternalTransaction;
use crate::starknet::business_logic::fact_state::contract_class_object::ContractState;
use crate::starknet::starknet_storage::CommitmentInfo;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct StarknetOsInput {
    pub contract_state_commitment_info: CommitmentInfo,
    pub contract_class_commitment_info: CommitmentInfo,
    pub deprecated_compiled_classes: HashMap<Felt, GenericDeprecatedCompiledClass>,
    pub compiled_classes: HashMap<Felt252, GenericCasmContractClass>,
    pub compiled_class_visited_pcs: HashMap<Felt252, Vec<Felt252>>,
    pub contracts: HashMap<Felt252, ContractState>,
    pub contract_address_to_class_hash: HashMap<Felt252, Felt252>,
    pub class_hash_to_compiled_class_hash: HashMap<Felt252, Felt252>,
    pub general_config: StarknetGeneralConfig,
    pub transactions: Vec<InternalTransaction>,
    pub declared_class_hash_to_component_hashes: HashMap<Felt252, Vec<Felt252>>,
    pub new_block_hash: Felt252,
    pub prev_block_hash: Felt252,
    pub full_output: bool,
}