use blockifier::execution::contract_class::ContractClass;
use blockifier::state::errors::StateError;
use blockifier::state::state_api::{StateReader, StateResult};
use starknet::core::types::BlockId;
use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce};
use starknet_api::state::StorageKey;
use starknet_types_core::felt::Felt;
use rpc_client::client::RpcClient;
use crate::utils::execute_coroutine;

pub struct AsyncRpcStateReader {
    rpc_client: RpcClient,
    block_id: BlockId,
}

impl AsyncRpcStateReader {
    pub fn new(rpc_client: RpcClient, block_id: BlockId) -> Self {
        Self { rpc_client, block_id }
    }
}

fn to_state_err<E: ToString>(e: E) -> StateError {
    StateError::StateReadError(e.to_string())
}

impl StateReader for AsyncRpcStateReader {
    fn get_storage_at(&self, contract_address: ContractAddress, key: StorageKey) -> StateResult<Felt> {
        execute_coroutine(self.get_storage_at_async(contract_address, key)).map_err(to_state_err)?
    }

    fn get_nonce_at(&self, contract_address: ContractAddress) -> StateResult<Nonce> {
        execute_coroutine(self.get_nonce_at_async(contract_address)).map_err(to_state_err)?
    }

    fn get_class_hash_at(&self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        execute_coroutine(self.get_class_hash_at_async(contract_address))
            .map_err(|e| StateError::StateReadError(e.to_string()))?
    }

    fn get_compiled_contract_class(&self, class_hash: ClassHash) -> StateResult<ContractClass> {
        execute_coroutine(self.get_compiled_contract_class_async(class_hash)).map_err(to_state_err)?
    }

    fn get_compiled_class_hash(&self, class_hash: ClassHash) -> StateResult<CompiledClassHash> {
        execute_coroutine(self.get_compiled_class_hash_async(class_hash)).map_err(to_state_err)?
    }
}