use std::error::Error;
use blockifier::blockifier::block::{pre_process_block, BlockNumberHashPair};
use blockifier::context::BlockContext;
use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::objects::TransactionExecutionInfo;
use blockifier::transaction::transaction_execution::Transaction;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_api::transaction::TransactionHash;
use starknet_core::types::Felt;
use arcane_os::config::STORED_BLOCK_HASH_BUFFER;

pub fn reexecute_transactions_with_blockifier<S: StateReader>(
    state: &mut CachedState<S>,
    block_context: &BlockContext,
    buffer_block_hash: Felt,
    txs: Vec<Transaction>,
) -> Result<Vec<TransactionExecutionInfo>, Box<dyn Error>> {
    let current_block_number = block_context.block_info().block_number;
    let buffer_block_number_and_hash = if current_block_number.0 >= STORED_BLOCK_HASH_BUFFER {
        Some(BlockNumberHashPair {
            number: starknet_api::block::BlockNumber(current_block_number.0 - STORED_BLOCK_HASH_BUFFER),
            hash: starknet_api::block::BlockHash(buffer_block_hash),
        })
    } else {
        None
    };
    // Block pre-processing.
    // Writes the hash of the (current_block_number - N) block under its block number in the dedicated
    // contract state, where N=STORED_BLOCK_HASH_BUFFER.
    // https://github.com/starkware-libs/sequencer/blob/ee6513d338011067e46c55db4aa6926c8e57650e/crates/blockifier/src/blockifier/block.rs#L110
    pre_process_block(state, buffer_block_number_and_hash, current_block_number)?;

    let n_txs = txs.len();
    let tx_execution_infos = txs
        .into_iter()
        .enumerate()
        .map(|(index, tx)| {
            let tx_hash = get_tx_hash(&tx);
            let tx_result = tx.execute(state, block_context, true, true);
            match tx_result {
                Err(e) => {
                    panic!("Transaction {:x} ({}/{}) failed in blockifier: {}", tx_hash.0, index + 1, n_txs, e);
                }
                Ok(info) => {
                    if info.is_reverted() {
                        log::warn!(
                            "Transaction {:x} ({}/{}) reverted: {:?}",
                            tx_hash.0,
                            index + 1,
                            n_txs,
                            info.revert_error
                        );
                        log::warn!("TransactionExecutionInfo: {:?}", info);
                    }
                    info
                }
            }
        })
        .collect();

    Ok(tx_execution_infos)
}

fn get_tx_hash(tx: &Transaction) -> TransactionHash {
    match tx {
        Transaction::AccountTransaction(account_tx) => match account_tx {
            AccountTransaction::Declare(declare_tx) => declare_tx.tx_hash,
            AccountTransaction::DeployAccount(deploy_tx) => deploy_tx.tx_hash,
            AccountTransaction::Invoke(invoke_tx) => invoke_tx.tx_hash,
        },
        Transaction::L1HandlerTransaction(l1_handler_tx) => l1_handler_tx.tx_hash,
    }
}
