use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::vec::IntoIter;
use blockifier::context::BlockContext;
use blockifier::execution::call_info::CallInfo;
use blockifier::execution::entry_point_execution::CallResult;
use blockifier::transaction::objects::TransactionExecutionInfo;
use cairo_vm::Felt252;
use cairo_vm::types::relocatable::Relocatable;
use tokio::sync::RwLock;
use crate::config::STORED_BLOCK_HASH_BUFFER;
use crate::execution::secp_handler::SecpSyscallProcessor;
use crate::io::input::StarknetOsInput;
use crate::starknet::core::os::kzg_manager::KzgManager;
use crate::starknet::starknet_storage::{CommitmentInfo, CommitmentInfoError, PerContractStorage};
use crate::storage::error::StorageError;

pub type ContractStorageMap<PCS> = HashMap<Felt252, PCS>;

pub struct ExecutionHelper<PCS>
where
    PCS: PerContractStorage,
{
    pub _prev_block_context: Option<BlockContext>,
    pub os_input: Option<Rc<StarknetOsInput>>,
    pub kzg_manager: KzgManager,
    // Pointer tx execution info
    pub tx_execution_info_iter: IntoIter<TransactionExecutionInfo>,
    // Tx info for transaction currently being executed
    pub tx_execution_info: Option<TransactionExecutionInfo>,
    // Pointer to the Cairo (deprecated) TxInfo struct
    // Must match the DeprecatedTxInfo pointer for system call validation in 'enter_tx'
    pub tx_info_ptr: Option<Relocatable>,
    // Pointer to the Cairo ExecutionInfo struct of the current call.
    // Must match the ExecutionInfo pointer for system call validation in 'enter_call'
    pub call_execution_info_ptr: Option<Relocatable>,
    // The block number and block hash of the (current_block_number - buffer) block, where
    // buffer=STORED_BLOCK_HASH_BUFFER.
    // It is the hash that is going to be written by this OS run.
    pub old_block_number_and_hash: Option<(Felt252, Felt252)>,
    // Iter for CallInfo
    pub call_iter: IntoIter<CallInfo>,
    // CallInfo for the call currently being executed
    pub call_info: Option<CallInfo>,
    // Iter to the results of the current call's internal calls
    pub result_iter: IntoIter<CallResult>,
    // Iter over contract addresses that were deployed during that call
    pub deployed_contracts_iter: IntoIter<Felt252>,
    // Iter to the read_values array consumed when tx code is executed
    pub execute_code_read_iter: IntoIter<Felt252>,
    // Per-contract storage
    pub storage_by_address: ContractStorageMap<PCS>,

    // Secp syscall processors.
    pub secp256k1_syscall_processor: SecpSyscallProcessor<ark_secp256k1::Config>,
    pub secp256r1_syscall_processor: SecpSyscallProcessor<ark_secp256r1::Config>,

    // Sha256 segments
    pub sha256_segment: Option<Relocatable>,
}

#[derive(Debug)]
pub struct ExecutionHelperWrapper<PCS: PerContractStorage> {
    pub execution_helper: Rc<RwLock<ExecutionHelper<PCS>>>,
}

impl<PCS> Clone for ExecutionHelperWrapper<PCS>
where
    PCS: PerContractStorage,
{
    fn clone(&self) -> Self {
        Self { execution_helper: self.execution_helper.clone() }
    }
}

impl<PCS> ExecutionHelperWrapper<PCS>
where
    PCS: PerContractStorage + 'static,
{
    pub fn new(
        contract_storage_map: ContractStorageMap<PCS>,
        tx_execution_infos: Vec<TransactionExecutionInfo>,
        block_context: &BlockContext,
        os_input: Option<Rc<StarknetOsInput>>,
        old_block_number_and_hash: (Felt252, Felt252),
    ) -> Self {
        // Block number and block hash (current_block_number - buffer) block buffer=STORED_BLOCK_HASH_BUFFER
        // Hash that is going to be written by this OS run
        let prev_block_context = block_context
            .block_info()
            .block_number
            .0
            .checked_sub(STORED_BLOCK_HASH_BUFFER)
            .map(|_| block_context.clone());

        Self {
            execution_helper: Rc::new(RwLock::new(ExecutionHelper {
                _prev_block_context: prev_block_context,
                os_input,
                kzg_manager: Default::default(),
                tx_execution_info_iter: tx_execution_infos.into_iter(),
                tx_execution_info: None,
                tx_info_ptr: None,
                call_iter: vec![].into_iter(),
                call_execution_info_ptr: None,
                old_block_number_and_hash: Some(old_block_number_and_hash),
                call_info: None,
                result_iter: vec![].into_iter(),
                deployed_contracts_iter: vec![].into_iter(),
                execute_code_read_iter: vec![].into_iter(),
                storage_by_address: contract_storage_map,
                secp256k1_syscall_processor: Default::default(),
                secp256r1_syscall_processor: Default::default(),
                sha256_segment: None,
            })),
        }
    }
}
