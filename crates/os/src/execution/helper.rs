use crate::config::STORED_BLOCK_HASH_BUFFER;
use crate::execution::secp_handler::SecpSyscallProcessor;
use crate::io::input::StarknetOsInput;
use crate::starknet::core::os::kzg_manager::KzgManager;
use crate::starknet::starknet_storage::PerContractStorage;
use blockifier::context::BlockContext;
use blockifier::execution::call_info::CallInfo;
use blockifier::execution::entry_point_execution::CallResult;
use blockifier::transaction::objects::TransactionExecutionInfo;
use cairo_vm::types::relocatable::Relocatable;
use cairo_vm::Felt252;
use std::collections::HashMap;
use std::rc::Rc;
use std::vec::IntoIter;
use tokio::sync::RwLock;

pub type ContractStorageMap<PCS> = HashMap<Felt252, PCS>;

pub struct ExecutionHelper<PCS>
where
    PCS: PerContractStorage,
{
    pub _prev_block_context: Option<BlockContext>,
    pub os_input: Option<Rc<StarknetOsInput>>,
    pub kzg_manager: KzgManager,
    pub tx_execution_info_iter: IntoIter<TransactionExecutionInfo>,
    pub tx_execution_info: Option<TransactionExecutionInfo>,
    pub tx_info_ptr: Option<Relocatable>,
    pub call_execution_info_ptr: Option<Relocatable>,
    pub old_block_number_and_hash: Option<(Felt252, Felt252)>,
    pub call_iter: IntoIter<CallInfo>,
    pub call_info: Option<CallInfo>,
    pub result_iter: IntoIter<CallResult>,
    pub deployed_contracts_iter: IntoIter<Felt252>,
    pub execute_code_read_iter: IntoIter<Felt252>,
    pub storage_by_address: ContractStorageMap<PCS>,
    pub secp256k1_syscall_processor: SecpSyscallProcessor<ark_secp256k1::Config>,
    pub secp256r1_syscall_processor: SecpSyscallProcessor<ark_secp256r1::Config>,
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
        Self {
            execution_helper: self.execution_helper.clone(),
        }
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
