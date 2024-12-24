use cairo_type_derive::FieldOffsetGetters;
use cairo_vm::types::relocatable::Relocatable;
use cairo_vm::Felt252;

#[derive(FieldOffsetGetters)]
pub struct TxInfo {
    /// The version of the transaction. It is fixed (currently, 1) in the OS, and should be
    /// signed by the account contract.
    /// This field allows invalidating old transactions, whenever the meaning of the other
    /// transaction fields is changed (in the OS).
    #[allow(unused)]
    pub version: Felt252,
    /// The account contract from which this transaction originates.
    #[allow(unused)]
    pub account_contract_address: Felt252,
    /// The max_fee field of the transaction.
    #[allow(unused)]
    pub max_fee: Felt252,
    /// The signature of the transaction.
    #[allow(unused)]
    pub signature_len: Felt252,
    #[allow(unused)]
    pub signature: Relocatable,
    /// The hash of the transaction.
    #[allow(unused)]
    pub transaction_hash: Felt252,
    /// The identifier of the chain.
    /// This field can be used to prevent replay of testnet transactions on mainnet.
    #[allow(unused)]
    pub chain_id: Felt252,
    /// The transaction's nonce.
    #[allow(unused)]
    pub nonce: Felt252,
}

#[allow(unused)]
#[derive(FieldOffsetGetters)]
pub struct StorageWrite {
    pub selector: Felt252,
    pub address: Felt252,
    pub value: Felt252,
}

#[allow(unused)]
#[derive(FieldOffsetGetters)]
pub struct StorageRead {
    pub request: StorageReadRequest,
    pub response: StorageReadResponse,
}

#[allow(unused)]
#[derive(FieldOffsetGetters)]
pub struct StorageReadRequest {
    pub selector: Felt252,
    pub address: Felt252,
}

#[allow(unused)]
#[derive(FieldOffsetGetters)]
pub struct StorageReadResponse {
    pub value: Felt252,
}

#[allow(unused)]
#[derive(FieldOffsetGetters)]
pub struct SecpNewResponse {
    #[allow(unused)]
    pub not_on_curve: Felt252,
    #[allow(unused)]
    pub ec_point: Relocatable,
}
