use std::collections::HashMap;
use blockifier::transaction::objects::TransactionExecutionInfo;
use cairo_vm::Felt252;
use num_bigint::BigInt;
use starknet_api::contract_address;
use starknet_api::core::ContractAddress;
use starknet_api::state::StorageKey;
use starknet_core::types::Felt;
use arcane_os::config::DEFAULT_STORAGE_TREE_HEIGHT;
use arcane_os::starkware_utils::commitment_tree::base_types::Height;
use rpc_client::client::RpcClient;
use rpc_client::pathfinder::client::ClientError;
use rpc_client::pathfinder::proofs::{ContractData, EdgePath, PathfinderClassProof, PathfinderProof, ProofVerificationError, TrieNode};
use crate::utils::get_all_accessed_keys;

pub(crate) async fn get_storage_proofs(
    client: &RpcClient,
    block_number: u64,
    tx_execution_infos: &[TransactionExecutionInfo],
    old_block_number: Felt,
) -> Result<HashMap<Felt, PathfinderProof>, ClientError> {
    let accessed_keys_by_address = {
        let mut keys = get_all_accessed_keys(tx_execution_infos);
        // We need to fetch the storage proof for the block hash contract
        keys.entry(contract_address!("0x1")).or_default().insert(old_block_number.try_into().unwrap());
        keys
    };

    let mut storage_proofs = HashMap::new();

    log::info!("Contracts we're fetching proofs for:");
    for (contract_address, storage_keys) in accessed_keys_by_address {
        log::info!("    Fetching proof for {}", contract_address.to_string());
        let contract_address_felt = *contract_address.key();
        let storage_proof =
            get_storage_proof_for_contract(client, contract_address, storage_keys.into_iter(), block_number).await?;
        storage_proofs.insert(contract_address_felt, storage_proof);
    }

    Ok(storage_proofs)
}

async fn get_storage_proof_for_contract<KeyIter: Iterator<Item = StorageKey>>(
    rpc_client: &RpcClient,
    contract_address: ContractAddress,
    storage_keys: KeyIter,
    block_number: u64,
) -> Result<PathfinderProof, ClientError> {
    let contract_address_felt = *contract_address.key();
    let keys: Vec<_> = storage_keys.map(|storage_key| *storage_key.key()).collect();

    let mut storage_proof =
        fetch_storage_proof_for_contract(rpc_client, contract_address_felt, &keys, block_number).await?;

    let contract_data = match &storage_proof.contract_data {
        None => {
            return Ok(storage_proof);
        }
        Some(contract_data) => contract_data,
    };
    let additional_keys = verify_storage_proof(contract_data, &keys);

    // Fetch additional proofs required to fill gaps in the storage trie that could make
    // the OS crash otherwise.
    if !additional_keys.is_empty() {
        let additional_proof =
            fetch_storage_proof_for_contract(rpc_client, contract_address_felt, &additional_keys, block_number).await?;

        storage_proof = merge_storage_proofs(vec![storage_proof, additional_proof]);
    }

    Ok(storage_proof)
}

async fn fetch_storage_proof_for_contract(
    rpc_client: &RpcClient,
    contract_address: Felt,
    keys: &[Felt],
    block_number: u64,
) -> Result<PathfinderProof, ClientError> {
    let storage_proof = if keys.is_empty() {
        rpc_client.pathfinder_rpc().get_proof(block_number, contract_address, &[]).await?
    } else {
        // The endpoint is limited to 100 keys at most per call
        const MAX_KEYS: usize = 100;
        let mut chunked_storage_proofs = Vec::new();
        for keys_chunk in keys.chunks(MAX_KEYS) {
            chunked_storage_proofs
                .push(rpc_client.pathfinder_rpc().get_proof(block_number, contract_address, keys_chunk).await?);
        }
        merge_storage_proofs(chunked_storage_proofs)
    };

    Ok(storage_proof)
}

fn merge_storage_proofs(proofs: Vec<PathfinderProof>) -> PathfinderProof {
    let class_commitment = proofs[0].class_commitment;
    let state_commitment = proofs[0].state_commitment;
    let contract_proof = proofs[0].contract_proof.clone();

    let contract_data = {
        let mut contract_data: Option<ContractData> = None;

        for proof in proofs {
            if let Some(data) = proof.contract_data {
                if let Some(contract_data) = contract_data.as_mut() {
                    contract_data.storage_proofs.extend(data.storage_proofs);
                } else {
                    contract_data = Some(data);
                }
            }
        }

        contract_data
    };

    PathfinderProof { class_commitment, state_commitment, contract_proof, contract_data }
}

fn verify_storage_proof(contract_data: &ContractData, keys: &[Felt]) -> Vec<Felt> {
    let mut additional_keys = vec![];
    if let Err(errors) = contract_data.verify(keys) {
        for error in errors {
            match error {
                ProofVerificationError::NonExistenceProof { key, height, proof } => {
                    if let Some(TrieNode::Edge { child: _, path }) = proof.last() {
                        if height.0 < DEFAULT_STORAGE_TREE_HEIGHT {
                            let modified_key = get_key_following_edge(key, height, path);
                            log::trace!(
                                "Fetching modified key {} for key {}",
                                modified_key.to_hex_string(),
                                key.to_hex_string()
                            );
                            additional_keys.push(modified_key);
                        }
                    }
                }
                _ => {
                    panic!("Proof verification failed: {}", error);
                }
            }
        }
    }

    additional_keys
}

fn get_key_following_edge(key: Felt, height: Height, edge_path: &EdgePath) -> Felt {
    assert!(height.0 < DEFAULT_STORAGE_TREE_HEIGHT);

    let shift = height.0;
    let clear_mask = ((BigInt::from(1) << edge_path.len) - BigInt::from(1)) << shift;
    let mask = edge_path.value.to_bigint() << shift;
    let new_key = (key.to_bigint() & !clear_mask) | mask;

    Felt::from(new_key)
}

pub(crate) async fn get_class_proofs(
    rpc_client: &RpcClient,
    block_number: u64,
    class_hashes: &[&Felt],
) -> Result<HashMap<Felt252, PathfinderClassProof>, ClientError> {
    let mut proofs: HashMap<Felt252, PathfinderClassProof> = HashMap::with_capacity(class_hashes.len());
    for class_hash in class_hashes {
        let proof = rpc_client.pathfinder_rpc().get_class_proof(block_number, class_hash).await?;
        proofs.insert(Felt252::from(**class_hash), proof);
    }

    Ok(proofs)
}