use std::collections::BTreeMap;
use blockifier::blockifier::block::GasPrices;
use blockifier::transaction::errors::TransactionExecutionError;
use starknet::core::types::{DeclareTransaction, DeployAccountTransaction, InvokeTransaction, ResourceBoundsMapping, Transaction, TransactionTraceWithHash};
use starknet::providers::ProviderError;
use starknet_api::StarknetApiError;
use thiserror::Error;
use arcane_os_type::arcane_core_addons::LegacyContractDecompressionError;
use arcane_os_type::error::ContractClassError;
use crate::utils::FeltConversionError;

#[derive(Error, Debug)]
pub enum ToBlockifierError {
    #[error("RPC Error: {0}")]
    RpcError(#[from] ProviderError),
    #[error("OS Contract Class Error: {0}")]
    StarknetContractClassError(#[from] ContractClassError),
    #[error("Blockifier Contract Class Error: {0}")]
    BlockifierContractClassError(#[from] blockifier::execution::errors::ContractClassError),
    #[error("Legacy Contract Decompression Error: {0}")]
    LegacyContractDecompressionError(#[from] LegacyContractDecompressionError),
    #[error("Starknet API Error: {0}")]
    StarknetApiError(#[from] StarknetApiError),
    #[error("Transaction Execution Error: {0}")]
    TransactionExecutionError(#[from] TransactionExecutionError),
    #[error("Felt Conversion Error: {0}")]
    FeltConversionError(#[from] FeltConversionError),
}

pub fn resource_bounds_core_to_api(
    resource_bounds: &ResourceBoundsMapping,
) -> starknet_api::transaction::ResourceBoundsMapping {
    starknet_api::transaction::ResourceBoundsMapping(BTreeMap::from([
        (
            starknet_api::transaction::Resource::L1Gas,
            starknet_api::transaction::ResourceBounds {
                max_amount: resource_bounds.l1_gas.max_amount,
                max_price_per_unit: resource_bounds.l1_gas.max_price_per_unit,
            },
        ),
        (
            starknet_api::transaction::Resource::L2Gas,
            starknet_api::transaction::ResourceBounds {
                max_amount: resource_bounds.l2_gas.max_amount,
                max_price_per_unit: resource_bounds.l2_gas.max_price_per_unit,
            },
        ),
    ]))
}

pub async fn starknet_rs_to_blockifier(
    sn_core_tx: &starknet::core::types::Transaction,
    trace: &TransactionTraceWithHash,
    gas_prices: &GasPrices,
    client: &RpcClient,
    block_number: u64,
) -> Result<blockifier::transaction::transaction_execution::Transaction, ToBlockifierError> {
    let blockifier_tx = match sn_core_tx {
        Transaction::Invoke(tx) => match tx {
            InvokeTransaction::V0(_) => unimplemented!("starknet_rs_to_blockifier with InvokeTransaction::V0"),
            InvokeTransaction::V1(tx) => invoke_v1_to_blockifier(tx)?,
            InvokeTransaction::V3(tx) => invoke_v3_to_blockifier(tx)?,
        },
        Transaction::Declare(tx) => match tx {
            DeclareTransaction::V0(_) => unimplemented!("starknet_rs_to_blockifier with DeclareTransaction::V0"),
            DeclareTransaction::V1(tx) => declare_v1_to_blockifier(tx, client, block_number).await?,
            DeclareTransaction::V2(tx) => declare_v2_to_blockifier(tx, client, block_number).await?,
            DeclareTransaction::V3(tx) => declare_v3_to_blockifier(tx, client, block_number).await?,
        },
        Transaction::L1Handler(tx) => l1_handler_to_blockifier(tx, trace, gas_prices)?,
        Transaction::DeployAccount(tx) => match tx {
            DeployAccountTransaction::V1(tx) => deploy_account_v1_to_blockifier(tx)?,
            DeployAccountTransaction::V3(tx) => deploy_account_v3_to_blockifier(tx)?,
        },

        Transaction::Deploy(_) => {
            unimplemented!("we do not plan to support deprecated deploy txs, only deploy_account")
        }
    };

    Ok(blockifier_tx)
}
