use arcane_os::io::InternalTransaction;
use cairo_vm::Felt252;
use rpc_replay::transaction::resource_bounds_core_to_api;
use starknet::core::types::{
    DataAvailabilityMode, DeclareTransaction, DeclareTransactionV0, DeclareTransactionV1,
    DeclareTransactionV2, DeclareTransactionV3, DeployAccountTransaction,
    DeployAccountTransactionV1, InvokeTransaction, InvokeTransactionV0, InvokeTransactionV1,
    InvokeTransactionV3, L1HandlerTransaction, Transaction,
};
use starknet_api::core::{calculate_contract_address, ClassHash};
use starknet_api::transaction::{Calldata, ContractAddressSalt};
use starknet_core::types::DeployAccountTransactionV3;
use std::sync::Arc;

const EXECUTE_ENTRY_POINT_FELT: Felt252 = Felt252::from_hex_unchecked(
    "0x15d40a3d6ca2ac30f4031e42be28da9b056fef9bb7357ac5e85627ee876e5ad",
);

fn da_to_felt(data_availability_mode: DataAvailabilityMode) -> Felt252 {
    match data_availability_mode {
        DataAvailabilityMode::L1 => Felt252::ZERO,
        DataAvailabilityMode::L2 => Felt252::ONE,
    }
}

pub(crate) fn starknet_rs_tx_to_internal_tx(tx: Transaction) -> InternalTransaction {
    match tx {
        Transaction::Invoke(invoke_tx) => invoke_tx_to_internal_tx(invoke_tx),
        Transaction::L1Handler(l1_handler_tx) => l1handler_to_internal_tx(l1_handler_tx),
        Transaction::Declare(declare_tx) => match declare_tx {
            DeclareTransaction::V0(tx) => declare_v0_to_internal_tx(tx),
            DeclareTransaction::V1(tx) => declare_v1_to_internal_tx(tx),
            DeclareTransaction::V2(tx) => declare_v2_to_internal_tx(tx),
            DeclareTransaction::V3(tx) => declare_v3_to_internal_tx(tx),
        },
        Transaction::Deploy(_deploy_tx) => {
            unimplemented!("we do not plan to support deprecated deploy txs, only deploy_account")
        }
        Transaction::DeployAccount(deploy_account_tx) => match deploy_account_tx {
            DeployAccountTransaction::V1(tx) => deploy_account_v1_to_internal_tx(tx),
            DeployAccountTransaction::V3(tx) => deploy_account_v3_to_internal_tx(tx),
        },
    }
}

fn l1handler_to_internal_tx(input: L1HandlerTransaction) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(input.transaction_hash),
        version: Some(Felt252::from(input.version)),
        contract_address: Some(Felt252::from(input.contract_address)),
        nonce: Some(Felt252::from(input.nonce)),
        entry_point_selector: Some(Felt252::from(input.entry_point_selector)),
        calldata: Some(
            input
                .calldata
                .into_iter()
                .map(|calldata| Felt252::from(calldata))
                .collect(),
        ),
        r#type: "L1_HANDLER".to_string(),
        ..Default::default()
    }
}

fn invoke_tx_to_internal_tx(invoke_tx: InvokeTransaction) -> InternalTransaction {
    let mut internal_tx = match invoke_tx {
        InvokeTransaction::V0(invoke_v0_tx) => invoke_tx_v0_to_internal_tx(invoke_v0_tx),
        InvokeTransaction::V1(invoke_v1_tx) => invoke_tx_v1_to_internal_tx(invoke_v1_tx),
        InvokeTransaction::V3(invoke_v3_tx) => invoke_tx_v3_to_internal_tx(invoke_v3_tx),
    };
    internal_tx.r#type = "INVOKE_FUNCTION".into();

    internal_tx
}

fn invoke_tx_v0_to_internal_tx(tx: InvokeTransactionV0) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(tx.transaction_hash),
        max_fee: Some(Felt252::from(tx.max_fee)),
        signature: Some(
            tx.signature
                .into_iter()
                .map(|signature| Felt252::from(signature))
                .collect(),
        ),
        contract_address: Some(Felt252::from(tx.contract_address)),
        entry_point_selector: Some(Felt252::from(tx.entry_point_selector)),
        calldata: Some(
            tx.calldata
                .into_iter()
                .map(|calldata| Felt252::from(calldata))
                .collect(),
        ),
        version: Some(Felt252::ZERO),
        ..Default::default()
    }
}

fn invoke_tx_v1_to_internal_tx(tx: InvokeTransactionV1) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(tx.transaction_hash),
        version: Some(Felt252::ONE),
        contract_address: Some(Felt252::from(tx.sender_address)),
        nonce: Some(Felt252::from(tx.nonce)),
        sender_address: Some(Felt252::from(tx.sender_address)),
        entry_point_selector: Some(EXECUTE_ENTRY_POINT_FELT),
        entry_point_type: Some("EXTERNAL".to_string()),
        signature: Some(
            tx.signature
                .into_iter()
                .map(|signature| Felt252::from(signature))
                .collect(),
        ),
        calldata: Some(
            tx.calldata
                .into_iter()
                .map(|calldata| Felt252::from(calldata))
                .collect(),
        ),
        r#type: "INVOKE_FUNCTION".to_string(),
        max_fee: Some(Felt252::from(tx.max_fee)),
        ..Default::default()
    }
}

fn invoke_tx_v3_to_internal_tx(tx: InvokeTransactionV3) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(tx.transaction_hash),
        sender_address: Some(Felt252::from(tx.sender_address)),
        signature: Some(tx.signature.into_iter().map(Felt252::from).collect()),
        nonce: Some(Felt252::from(tx.nonce)),
        resource_bounds: Some(resource_bounds_core_to_api(&tx.resource_bounds)),
        tip: Some(Felt252::from(tx.tip)),
        paymaster_data: Some(tx.paymaster_data.into_iter().map(Felt252::from).collect()),
        account_deployment_data: Some(
            tx.account_deployment_data
                .into_iter()
                .map(Felt252::from)
                .collect(),
        ),
        nonce_data_availability_mode: Some(da_to_felt(tx.nonce_data_availability_mode)),
        fee_data_availability_mode: Some(da_to_felt(tx.fee_data_availability_mode)),
        version: Some(Felt252::THREE),
        contract_address: Some(Felt252::from(tx.sender_address)),
        entry_point_selector: Some(EXECUTE_ENTRY_POINT_FELT),
        entry_point_type: Some("EXTERNAL".to_string()),
        calldata: Some(tx.calldata.into_iter().map(Felt252::from).collect()),
        ..Default::default()
    }
}

fn declare_v0_to_internal_tx(input: DeclareTransactionV0) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(input.transaction_hash),
        sender_address: Some(Felt252::from(input.sender_address)),
        max_fee: Some(Felt252::from(input.max_fee)),
        signature: Some(input.signature.into_iter().map(Felt252::from).collect()),
        class_hash: Some(Felt252::from(input.class_hash)),
        r#type: "DECLARE".to_string(),
        version: Some(Felt252::ZERO),
        ..Default::default()
    }
}

fn declare_v1_to_internal_tx(input: DeclareTransactionV1) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(input.transaction_hash),
        sender_address: Some(Felt252::from(input.sender_address)),
        max_fee: Some(Felt252::from(input.max_fee)),
        signature: Some(input.signature.into_iter().map(Felt252::from).collect()),
        nonce: Some(Felt252::from(input.nonce)),
        class_hash: Some(Felt252::from(input.class_hash)),
        r#type: "DECLARE".to_string(),
        version: Some(Felt252::ONE),
        ..Default::default()
    }
}

fn declare_v2_to_internal_tx(input: DeclareTransactionV2) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(input.transaction_hash),
        sender_address: Some(Felt252::from(input.sender_address)),
        compiled_class_hash: Some(Felt252::from(input.compiled_class_hash)),
        max_fee: Some(Felt252::from(input.max_fee)),
        signature: Some(input.signature.into_iter().map(Felt252::from).collect()),
        nonce: Some(Felt252::from(input.nonce)),
        class_hash: Some(Felt252::from(input.class_hash)),
        r#type: "DECLARE".to_string(),
        version: Some(Felt252::TWO),
        ..Default::default()
    }
}

fn declare_v3_to_internal_tx(input: DeclareTransactionV3) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(input.transaction_hash),
        sender_address: Some(Felt252::from(input.sender_address)),
        compiled_class_hash: Some(Felt252::from(input.compiled_class_hash)),
        signature: Some(input.signature.into_iter().map(Felt252::from).collect()),
        nonce: Some(Felt252::from(input.nonce)),
        class_hash: Some(Felt252::from(input.class_hash)),
        resource_bounds: Some(resource_bounds_core_to_api(&input.resource_bounds)),
        tip: Some(Felt252::from(input.tip)),
        paymaster_data: Some(
            input
                .paymaster_data
                .into_iter()
                .map(Felt252::from)
                .collect(),
        ),
        account_deployment_data: Some(
            input
                .account_deployment_data
                .into_iter()
                .map(Felt252::from)
                .collect(),
        ),
        nonce_data_availability_mode: Some(da_to_felt(input.nonce_data_availability_mode)),
        fee_data_availability_mode: Some(da_to_felt(input.fee_data_availability_mode)),
        r#type: "DECLARE".to_string(),
        version: Some(Felt252::THREE),
        ..Default::default()
    }
}

fn deploy_account_v1_to_internal_tx(input: DeployAccountTransactionV1) -> InternalTransaction {
    let entry_point_selector = Some(Felt252::ZERO);
    InternalTransaction {
        hash_value: Felt252::from(input.transaction_hash),
        max_fee: Some(Felt252::from(input.max_fee)),
        signature: Some(input.signature.into_iter().map(Felt252::from).collect()),
        nonce: Some(Felt252::from(input.nonce)),
        contract_address_salt: Some(Felt252::from(input.contract_address_salt)),
        constructor_calldata: Some(
            input
                .constructor_calldata
                .clone()
                .into_iter()
                .map(Felt252::from)
                .collect(),
        ),
        class_hash: Some(Felt252::from(input.class_hash)),
        r#type: "DEPLOY_ACCOUNT".to_string(),
        version: Some(Felt252::ONE),
        entry_point_selector,
        contract_address: Some(Felt252::from_bytes_be(
            &calculate_contract_address(
                ContractAddressSalt(input.contract_address_salt),
                ClassHash(input.class_hash),
                &Calldata(Arc::new(input.constructor_calldata)),
                Default::default(),
            )
            .unwrap()
            .0
            .key()
            .to_bytes_be(),
        )),
        ..Default::default()
    }
}

pub fn deploy_account_v3_to_internal_tx(input: DeployAccountTransactionV3) -> InternalTransaction {
    InternalTransaction {
        hash_value: Felt252::from(input.transaction_hash),
        signature: Some(input.signature.into_iter().map(Felt252::from).collect()),
        nonce: Some(Felt252::from(input.nonce)),
        contract_address_salt: Some(Felt252::from(input.contract_address_salt)),
        constructor_calldata: Some(
            input
                .constructor_calldata
                .into_iter()
                .map(Felt252::from)
                .collect(),
        ),
        class_hash: Some(Felt252::from(input.class_hash)),
        resource_bounds: Some(resource_bounds_core_to_api(&input.resource_bounds)),
        tip: Some(Felt252::from(input.tip)),
        paymaster_data: Some(
            input
                .paymaster_data
                .into_iter()
                .map(Felt252::from)
                .collect(),
        ),
        nonce_data_availability_mode: Some(da_to_felt(input.nonce_data_availability_mode)),
        fee_data_availability_mode: Some(da_to_felt(input.fee_data_availability_mode)),
        r#type: "DEPLOY_ACCOUNT".to_string(),
        version: Some(Felt252::THREE),
        ..Default::default()
    }
}
