mod types;
mod state_utils;
mod utils;

use blockifier::state::cached_state::CachedState;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use starknet::core::types::{BlockId, MaybePendingBlockWithTxs};
use starknet::providers::{JsonRpcClient, Provider, ProviderError, Url};
use starknet::providers::jsonrpc::HttpTransport;
use starknet_api::StarknetApiError;
use thiserror::Error;
use arcane_os::config::STORED_BLOCK_HASH_BUFFER;
use arcane_os::error::ArcaneError;
use arcane_os::io::output::StarknetOsOutput;
use arcane_os::starkware_utils::commitment_tree::error::TreeError;
use arcane_os_type::arcane_core_addons::LegacyContractDecompressionError;
use arcane_os_type::error::ContractClassError;
use rpc_replay::block_context::build_block_context;
use rpc_replay::rpc_state_reader::AsyncRpcStateReader;
use rpc_replay::transaction::ToBlockifierError;
use rpc_replay::utils::FeltConversionError;
use crate::state_utils::get_formatted_state_update;
use crate::types::starknet_rs_tx_to_internal_tx;

#[derive(Debug, Error)]
pub enum ProveBlockError {
    #[error("RPC Error: {0}")]
    RpcError(#[from] ProviderError),
    #[error("Re-Execution Error: {0}")]
    ReExecutionError(#[from] Box<dyn std::error::Error>),
    #[error("Tree Error: {0}")]
    TreeError(#[from] TreeError),
    #[error("Contract Class Error: {0}")]
    ContractClassError(#[from] ContractClassError),
    #[error("ArcaneError: {0}")]
    ArcaneError(#[from] ArcaneError),
    #[error("Legacy class decompression Error: {0}")]
    LegacyContractDecompressionError(#[from] LegacyContractDecompressionError),
    #[error("Starknet API Error: {0}")]
    StarknetApiError(StarknetApiError),
    #[error("To Blockifier Error: {0}")]
    ToBlockifierError(#[from] ToBlockifierError),
    #[error("Felt Conversion Error: {0}")]
    FeltConversionError(#[from] FeltConversionError),
}

pub async fn prove_block(
    complied_os: &[u8],
    block_number: u64,
    rpc_provider: &str,
    layout: &str,
    full_output: bool
) -> Result<(CairoPie, StarknetOsOutput), ProveBlockError> {
    // Create Madara Provider
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_provider).unwrap()));

    // Chain id
    let chain_id = provider.chain_id().await?.to_string();

    // Build block with txs
    let block_id = BlockId::Number(block_number);
    let previous_block_id = BlockId::Number(block_number - 1);
    let old_block_number = if block_number <= STORED_BLOCK_HASH_BUFFER { 0 } else { block_number - STORED_BLOCK_HASH_BUFFER };
    let old_block_id = BlockId::Number(old_block_number);

    let block_with_txs = match provider.get_block_with_txs(block_id).await? {
        MaybePendingBlockWithTxs::Block(block_with_txs) => block_with_txs,
        MaybePendingBlockWithTxs::PendingBlock(_) => {
            panic!("Block is still pending!")
        }
    };

    let previous_block_with_txs = match provider.get_block_with_txs(previous_block_id).await? {
        MaybePendingBlockWithTxs::Block(block_with_txs) => block_with_txs,
        MaybePendingBlockWithTxs::PendingBlock(_) => {
            panic!("Block is still pending!")
        }
    };

    let old_block_with_txs_hashes = match provider.get_block_with_tx_hashes(old_block_id).await? {
        MaybePendingBlockWithTxs::Block(block_with_txs) => block_with_txs,
        MaybePendingBlockWithTxs::PendingBlock(_) => {
            panic!("Block is still pending!")
        }
    };

    let block_context = build_block_context(chain_id, &block_with_txs)?;

    let (processed_state_update, traces) = get_formatted_state_update(&provider, previous_block_id, block_id).await?;
    let class_hash_to_compiled_class_hash = processed_state_update.class_hash_to_compiled_class_hash;

    let blockifier_state_reader = AsyncRpcStateReader::new(provider, previous_block_id);
    let mut blockifier_state = CachedState::new(blockifier_state_reader);

    assert_eq!(block_with_txs.transactions.len(), traces.len(), "Transactions and traces must have the same length");

    let mut txs = Vec::new();
    for (tx, trace) in block_with_txs.transactions.iter().zip(traces.iter()) {
        let transaction =
    }

    Ok(())
}