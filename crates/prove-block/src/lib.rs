#![feature(async_iter_from_iter)]

pub mod types;
pub mod state_utils;
pub mod utils;
pub mod reexecute;
pub mod rpc_utils;

use std::async_iter::FromIter;
use std::collections::HashMap;
use std::rc::Rc;
use crate::reexecute::{format_commitment_facts, reexecute_transactions_with_blockifier, ProverPerContractStorage};
use crate::state_utils::get_formatted_state_update;
use arcane_os::config::{StarknetGeneralConfig, StarknetOsConfig, STORED_BLOCK_HASH_BUFFER};
use arcane_os::error::ArcaneError;
use arcane_os::io::output::StarknetOsOutput;
use arcane_os::starkware_utils::commitment_tree::error::TreeError;
use arcane_os_type::arcane_core_addons::LegacyContractDecompressionError;
use arcane_os_type::error::ContractClassError;
use blockifier::state::cached_state::CachedState;
use cairo_vm::Felt252;
use starknet_api::core::ChainId;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use rpc_replay::block_context::build_block_context;
use rpc_replay::rpc_state_reader::AsyncRpcStateReader;
use rpc_replay::transaction::{starknet_rs_to_blockifier, ToBlockifierError};
use rpc_replay::utils::FeltConversionError;
use starknet::core::types::{BlockId, MaybePendingBlockWithTxs};
use starknet::providers::{Provider, ProviderError};
use starknet_api::StarknetApiError;
use starknet_core::types::StarknetError;
use starknet_types_core::felt::Felt;
use thiserror::Error;
use arcane_os::crypto::pedersen::PedersenHash;
use arcane_os::crypto::poseidon::PoseidonHash;
use arcane_os::execution::helper::{ContractStorageMap, ExecutionHelperWrapper};
use arcane_os::io::input::StarknetOsInput;
use arcane_os::run_os;
use arcane_os::starknet::business_logic::fact_state::contract_class_object::ContractState;
use arcane_os::starknet::starknet_storage::CommitmentInfo;
use arcane_os::starkware_utils::commitment_tree::base_types::Height;
use arcane_os::starkware_utils::commitment_tree::patricia_tree::patricia_tree::PatriciaTree;
use rpc_client::client::RpcClient;
use rpc_client::pathfinder::proofs::{PathfinderClassProof, ProofVerificationError};
use crate::rpc_utils::{get_class_proofs, get_storage_proofs};

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
    let provider = RpcClient::new(rpc_provider);

    // Chain id
    let chain_id = provider.starknet_rpc().chain_id().await?.to_string();

    // Build block with txs
    let block_id = BlockId::Number(block_number);
    let previous_block_id = BlockId::Number(block_number - 1);
    let old_block_number = if block_number <= STORED_BLOCK_HASH_BUFFER { 0 } else { block_number - STORED_BLOCK_HASH_BUFFER };
    let old_block_id = BlockId::Number(old_block_number);

    let block_with_txs = match provider.starknet_rpc().get_block_with_txs(block_id).await? {
        MaybePendingBlockWithTxs::Block(block_with_txs) => block_with_txs,
        MaybePendingBlockWithTxs::PendingBlock(_) => {
            panic!("Block is still pending!")
        }
    };

    let previous_block_with_txs = match provider.starknet_rpc().get_block_with_txs(previous_block_id).await? {
        MaybePendingBlockWithTxs::Block(block_with_txs) => block_with_txs,
        MaybePendingBlockWithTxs::PendingBlock(_) => {
            panic!("Block is still pending!")
        }
    };

    let old_block_with_txs_hashes = match provider.starknet_rpc().get_block_with_tx_hashes(old_block_id).await? {
        MaybePendingBlockWithTxs::Block(block_with_txs) => block_with_txs,
        MaybePendingBlockWithTxs::PendingBlock(_) => {
            panic!("Block is still pending!")
        }
    };
    let old_block_hash = old_block_with_txs_hashes.block_hash;

    let block_context = build_block_context(chain_id.clone(), &block_with_txs)?;

    let (processed_state_update, traces) = get_formatted_state_update(&provider, previous_block_id, block_id).await?;
    let class_hash_to_compiled_class_hash = processed_state_update.class_hash_to_compiled_class_hash;

    let blockifier_state_reader = AsyncRpcStateReader::new(provider.clone(), previous_block_id);
    let mut blockifier_state = CachedState::new(blockifier_state_reader);

    assert_eq!(block_with_txs.transactions.len(), traces.len(), "Transactions and traces must have the same length");

    let mut txs = Vec::new();
    for (tx, trace) in block_with_txs.transactions.iter().zip(traces.iter()) {
        let transaction = starknet_rs_to_blockifier(tx, trace, &block_context.block_info().gas_prices, &provider, block_number).await?;
        txs.push(transaction);
    }

    let tx_execution_infos =
        reexecute_transactions_with_blockifier(&mut blockifier_state, &block_context, old_block_hash, txs)?;

    let storage_proofs = get_storage_proofs(&provider, block_number, &tx_execution_infos, Felt::from(old_block_number))
        .await
        .expect("Failed to fetch storage proofs");

    let previous_storage_proofs =
        get_storage_proofs(&provider, block_number - 1, &tx_execution_infos, Felt::from(old_block_number))
            .await
            .expect("Failed to fetch storage proofs");

    let default_general_config = StarknetGeneralConfig::default();

    let general_config = StarknetGeneralConfig {
        starknet_os_config: StarknetOsConfig {
            chain_id: ChainId::Other(chain_id),
            fee_token_address: block_context.chain_info().fee_token_addresses.strk_fee_token_address,
            deprecated_fee_token_address: block_context.chain_info().fee_token_addresses.eth_fee_token_address,
        },
        ..default_general_config
    };

    let mut contract_states = HashMap::new();
    let mut contract_storages = ContractStorageMap::new();
    let mut contract_address_to_class_hash = HashMap::new();

    for (contract_address, storage_proof) in storage_proofs.clone() {
        let previous_storage_proof =
            previous_storage_proofs.get(&contract_address).expect("failed to find previous storage proof");
        let contract_storage_root = previous_storage_proof
            .contract_data
            .as_ref()
            .map(|contract_data| contract_data.root)
            .unwrap_or(Felt::ZERO)
            .into();

        log::debug!(
            "Storage root 0x{:x} for contract 0x{:x}",
            Into::<Felt252>::into(contract_storage_root),
            contract_address
        );

        let previous_tree = PatriciaTree { root: contract_storage_root, height: Height(251) };

        let contract_storage = ProverPerContractStorage::new(
            provider.clone(),
            previous_block_id,
            Felt252::from(contract_address),
            previous_tree.root.into(),
            storage_proof,
            previous_storage_proof.clone(),
        )?;
        contract_storages.insert(Felt252::from(contract_address), contract_storage);

        let (previous_class_hash, previous_nonce) = if [Felt252::ZERO, Felt252::ONE].contains(&Felt252::from(contract_address),) {
            (Felt252::ZERO, Felt252::ZERO)
        } else {
            let previous_class_hash =
                match provider.starknet_rpc().get_class_hash_at(previous_block_id, contract_address).await {
                    Ok(class_hash) => Ok(class_hash),
                    Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => Ok(Felt::ZERO),
                    Err(e) => Err(e),
                }?;

            let previous_nonce = match provider.starknet_rpc().get_nonce(previous_block_id, contract_address).await {
                Ok(nonce) => Ok(nonce),
                Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => Ok(Felt::ZERO),
                Err(e) => Err(e),
            }?;

            let class_hash = provider.starknet_rpc().get_class_hash_at(block_id, contract_address).await?;
            contract_address_to_class_hash.insert(contract_address, class_hash);

            (Felt252::from(previous_class_hash), Felt252::from(previous_nonce))
        };

        let contract_state = ContractState {
            contract_hash: previous_class_hash.to_bytes_be().to_vec(),
            storage_commitment_tree: previous_tree,
            nonce: previous_nonce,
        };

        contract_states.insert(contract_address, contract_state);
    }

    let compiled_classes = processed_state_update.compiled_classes;
    let deprecated_compiled_classes = processed_state_update.deprecated_compiled_classes;
    let declared_class_hash_component_hashes: HashMap<_, _> = processed_state_update
        .declared_class_hash_component_hashes
        .into_iter()
        .map(|(class_hash, component_hashes)| (class_hash, component_hashes.to_vec()))
        .collect();

    // query storage proofs for each accessed contract
    let class_hashes: Vec<&Felt> = class_hash_to_compiled_class_hash.keys().collect();
    // TODO: we fetch proofs here for block-1, but we probably also need to fetch at the current
    //       block, likely for contracts that are deployed in this block
    let class_proofs =
        get_class_proofs(&provider, block_number, &class_hashes[..]).await.expect("Failed to fetch class proofs");
    let previous_class_proofs = get_class_proofs(&provider, block_number - 1, &class_hashes[..])
        .await
        .expect("Failed to fetch previous class proofs");

    let visited_pcs: HashMap<Felt, Vec<Felt252>> = blockifier_state
        .visited_pcs
        .iter()
        .map(|(class_hash, visited_pcs)| {
            (class_hash.0, visited_pcs.iter().copied().map(Felt252::from).collect::<Vec<_>>())
        }).collect();

    // We can extract data from any storage proof, use the one of the block hash contract
    let block_hash_storage_proof =
        storage_proofs.get(&Felt::ONE).expect("there should be a storage proof for the block hash contract");
    let previous_block_hash_storage_proof = previous_storage_proofs
        .get(&Felt::ONE)
        .expect("there should be a previous storage proof for the block hash contract");

    // The root of the class commitment tree for previous and current block
    // Using requested storage proof instead of getting them from class proofs
    // If the block doesn't contain transactions, `class_proofs` will be empty
    // Pathfinder will send a None on class_commitment when the tree is not initialized, ie, root is zero
    let updated_root = block_hash_storage_proof.class_commitment.unwrap_or(Felt::ZERO);
    let previous_root = previous_block_hash_storage_proof.class_commitment.unwrap_or(Felt::ZERO);

    // On devnet and until block 10, the storage_root_idx might be None and that means that contract_proof is empty
    let previous_contract_trie_root = match previous_block_hash_storage_proof.contract_proof.first() {
        Some(proof) => proof.hash::<PedersenHash>(),
        None => Felt::ZERO,
    };
    let current_contract_trie_root = match block_hash_storage_proof.contract_proof.first() {
        Some(proof) => proof.hash::<PedersenHash>(),
        None => Felt::ZERO,
    };

    let previous_contract_proofs: Vec<_> =
        previous_storage_proofs.values().map(|proof| proof.contract_proof.clone()).collect();
    let previous_state_commitment_facts = format_commitment_facts::<PedersenHash>(&previous_contract_proofs);
    let current_contract_proofs: Vec<_> = storage_proofs.values().map(|proof| proof.contract_proof.clone()).collect();
    let current_state_commitment_facts = format_commitment_facts::<PedersenHash>(&current_contract_proofs);

    let global_state_commitment_facts: HashMap<_, _> =
        previous_state_commitment_facts.into_iter().chain(current_state_commitment_facts).collect();

    let contract_state_commitment_info = CommitmentInfo {
        previous_root: Felt252::from(previous_contract_trie_root),
        updated_root: Felt252::from(current_contract_trie_root),
        tree_height: 251,
        commitment_facts: global_state_commitment_facts,
    };

    let contract_class_commitment_info =
        compute_class_commitment(&previous_class_proofs, &class_proofs, previous_root, updated_root);

    let os_input = Rc::new(StarknetOsInput {
        contract_state_commitment_info,
        contract_class_commitment_info,
        deprecated_compiled_classes,
        compiled_classes,
        compiled_class_visited_pcs: visited_pcs,
        contracts: contract_states,
        contract_address_to_class_hash,
        class_hash_to_compiled_class_hash,
        general_config,
        transactions,
        declared_class_hash_to_component_hashes: declared_class_hash_component_hashes,
        new_block_hash: block_with_txs.block_hash,
        prev_block_hash: previous_block_with_txs.block_hash,
        full_output,
    });
    let execution_helper = ExecutionHelperWrapper::<ProverPerContractStorage>::new(
        contract_storages,
        tx_execution_infos,
        &block_context,
        Some(os_input.clone()),
        (Felt252::from(old_block_number), Felt252::from(old_block_hash)),
    );

    Ok(run_os(complied_os, layout, os_input, block_context, execution_helper)?)
}

fn compute_class_commitment(
    previous_class_proofs: &HashMap<Felt252, PathfinderClassProof>,
    class_proofs: &HashMap<Felt252, PathfinderClassProof>,
    previous_root: Felt,
    updated_root: Felt,
) -> CommitmentInfo {
    for (class_hash, previous_class_proof) in previous_class_proofs {
        if let Err(e) = previous_class_proof.verify(*class_hash) {
            match e {
                ProofVerificationError::NonExistenceProof { .. } => {}
                _ => panic!("Previous class proof verification failed"),
            }
        }
    }

    for (class_hash, class_proof) in class_proofs {
        if let Err(e) = class_proof.verify(*class_hash) {
            match e {
                ProofVerificationError::NonExistenceProof { .. } => {}
                _ => panic!("Current class proof verification failed"),
            }
        }
    }

    let previous_class_proofs: Vec<_> = previous_class_proofs.values().cloned().collect();
    let class_proofs: Vec<_> = class_proofs.values().cloned().collect();

    let previous_class_proofs: Vec<_> = previous_class_proofs.into_iter().map(|proof| proof.class_proof).collect();
    let class_proofs: Vec<_> = class_proofs.into_iter().map(|proof| proof.class_proof).collect();

    let previous_class_commitment_facts = format_commitment_facts::<PoseidonHash>(&previous_class_proofs);
    let current_class_commitment_facts = format_commitment_facts::<PoseidonHash>(&class_proofs);

    let class_commitment_facts: HashMap<_, _> =
        previous_class_commitment_facts.into_iter().chain(current_class_commitment_facts).collect();

    CommitmentInfo { previous_root, updated_root, tree_height: 251, commitment_facts: class_commitment_facts }
}
