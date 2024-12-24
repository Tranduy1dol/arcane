use cairo_type_derive::FieldOffsetGetters;
use cairo_vm::types::relocatable::Relocatable;
use cairo_vm::Felt252;

#[allow(unused)]
#[derive(FieldOffsetGetters)]
pub struct CallContractResponse {
    retdata_start: Felt252,
    retdata_end: Felt252,
}

#[derive(FieldOffsetGetters)]
pub struct DeployResponse {
    #[allow(unused)]
    pub contract_address: Felt252,
    #[allow(unused)]
    pub constructor_retdata_start: Relocatable,
    #[allow(unused)]
    pub constructor_retdata_end: Relocatable,
}

#[allow(unused)]
#[derive(FieldOffsetGetters)]
pub struct StorageWriteRequest {
    pub address_domain: Felt252,
    pub key: Felt252,
    pub value: Felt252,
}

#[derive(FieldOffsetGetters)]
pub struct StorageReadRequest {
    address_domain: Felt252,
    key: Felt252,
}
