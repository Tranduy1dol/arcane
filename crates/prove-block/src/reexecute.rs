use std::collections::HashMap;
use std::error::Error;
use blockifier::blockifier::block::{pre_process_block, BlockNumberHashPair};
use blockifier::context::BlockContext;
use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::objects::TransactionExecutionInfo;
use blockifier::transaction::transaction_execution::Transaction;
use blockifier::transaction::transactions::ExecutableTransaction;
use cairo_vm::Felt252;
use starknet::providers::{Provider, ProviderError};
use starknet_api::transaction::TransactionHash;
use starknet_core::types::{BlockId, StarknetError};
use starknet_types_core::felt::Felt;
use arcane_os::config::{DEFAULT_STORAGE_TREE_HEIGHT, STORED_BLOCK_HASH_BUFFER};
use arcane_os::crypto::pedersen::PedersenHash;
use arcane_os::starknet::starknet_storage::{CommitmentInfo, CommitmentInfoError, PerContractStorage};
use arcane_os::starkware_utils::commitment_tree::base_types::{Length, NodePath, TreeIndex};
use arcane_os::starkware_utils::commitment_tree::error::TreeError;
use arcane_os::starkware_utils::commitment_tree::inner_node_fact::InnerNodeFact;
use arcane_os::starkware_utils::commitment_tree::patricia_tree::nodes::{BinaryNodeFact, EdgeNodeFact};
use arcane_os::storage::dict_storage::DictStorage;
use arcane_os::storage::storage::{Fact, HashFunctionType};
use rpc_client::client::RpcClient;
use rpc_client::pathfinder::proofs::{ContractData, PathfinderProof, TrieNode};

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

pub(crate) fn format_commitment_facts<H: HashFunctionType>(
    trie_nodes: &[Vec<TrieNode>],
) -> HashMap<Felt252, Vec<Felt252>> {
    let mut facts = HashMap::new();

    for nodes in trie_nodes {
        for node in nodes {
            let (key, fact_as_tuple) = match node {
                TrieNode::Binary { left, right } => {
                    let fact = BinaryNodeFact::new((*left).into(), (*right).into())
                        .expect("storage proof endpoint gave us an invalid binary node");

                    // TODO: the hash function should probably be split from the Fact trait.
                    //       we use a placeholder for the Storage trait in the meantime.
                    let node_hash = Felt252::from(<BinaryNodeFact as Fact<DictStorage, H>>::hash(&fact));
                    let fact_as_tuple = <BinaryNodeFact as InnerNodeFact<DictStorage, H>>::to_tuple(&fact);

                    (node_hash, fact_as_tuple)
                }
                TrieNode::Edge { child, path } => {
                    let fact = EdgeNodeFact::new((*child).into(), NodePath(path.value.to_biguint()), Length(path.len))
                        .expect("storage proof endpoint gave us an invalid edge node");
                    // TODO: the hash function should probably be split from the Fact trait.
                    //       we use a placeholder for the Storage trait in the meantime.
                    let node_hash = Felt252::from(<EdgeNodeFact as Fact<DictStorage, H>>::hash(&fact));
                    let fact_as_tuple = <EdgeNodeFact as InnerNodeFact<DictStorage, H>>::to_tuple(&fact);

                    (node_hash, fact_as_tuple)
                }
            };

            let fact_as_tuple_of_felts: Vec<_> = fact_as_tuple.into_iter().map(Felt252::from).collect();
            facts.insert(key, fact_as_tuple_of_felts);
        }
    }

    facts
}

pub(crate) struct ProverPerContractStorage {
    rpc_client: RpcClient,
    block_id: BlockId,
    contract_address: Felt252,
    previous_tree_root: Felt252,
    storage_proof: PathfinderProof,
    previous_storage_proof: PathfinderProof,
    ongoing_storage_changes: HashMap<TreeIndex, Felt252>,
}

impl ProverPerContractStorage {
    pub fn new(
        rpc_client: RpcClient,
        block_id: BlockId,
        contract_address: Felt252,
        previous_tree_root: Felt252,
        storage_proof: PathfinderProof,
        previous_storage_proof: PathfinderProof,
    ) -> Result<Self, TreeError> {
        Ok(Self {
            rpc_client,
            block_id,
            contract_address,
            previous_tree_root,
            storage_proof,
            previous_storage_proof,
            ongoing_storage_changes: Default::default(),
        })
    }
}

impl PerContractStorage for ProverPerContractStorage {
    async fn compute_commitment(&mut self) -> Result<CommitmentInfo, CommitmentInfoError> {
        // TODO: error code
        let contract_data = match self.storage_proof.contract_data.as_ref() {
            None => &ContractData::default(),
            Some(data) => data,
        };

        let updated_root = Felt252::from(contract_data.root);

        let commitment_facts = format_commitment_facts::<PedersenHash>(&contract_data.storage_proofs);

        let previous_commitment_facts = match &self.previous_storage_proof.contract_data {
            None => HashMap::default(),
            Some(previous_contract_data) => {
                format_commitment_facts::<PedersenHash>(&previous_contract_data.storage_proofs)
            }
        };

        let commitment_facts = commitment_facts.into_iter().chain(previous_commitment_facts.into_iter()).collect();

        Ok(CommitmentInfo {
            previous_root: self.previous_tree_root,
            updated_root,
            tree_height: DEFAULT_STORAGE_TREE_HEIGHT as usize,
            commitment_facts,
        })
    }

    async fn read(&mut self, key: TreeIndex) -> Option<Felt252> {
        if let Some(value) = self.ongoing_storage_changes.get(&key) {
            Some(*value)
        } else {
            let key_felt = Felt252::from(key.clone());
            let value = match self
                .rpc_client
                .starknet_rpc()
                .get_storage_at(self.contract_address, key_felt, self.block_id)
                .await
            {
                Ok(value) => Ok(Felt252::from(value)),
                Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => Ok(Felt252::ZERO),
                Err(e) => Err(e),
            }
                .unwrap();
            self.ongoing_storage_changes.insert(key, value);
            Some(value)
        }
    }

    fn write(&mut self, key: TreeIndex, value: Felt252) {
        self.ongoing_storage_changes.insert(key, value);
    }
}