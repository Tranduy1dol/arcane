use std::rc::Rc;
use blockifier::blockifier::block::BlockInfo;
use cairo_vm::types::relocatable::Relocatable;
use tokio::sync::RwLock;
use crate::execution::helper::ExecutionHelperWrapper;
use crate::starknet::starknet_storage::PerContractStorage;

#[derive(Debug)]
pub struct DeprecatedOsSyscallHandler<PCS>
where
    PCS: PerContractStorage,
{
    pub exec_wrapper: ExecutionHelperWrapper<PCS>,
    pub syscall_ptr: Relocatable,
    block_info: BlockInfo,
}

#[derive(Debug)]
pub struct DeprecatedOsSyscallHandlerWrapper<PCS: PerContractStorage>
where
    PCS: PerContractStorage,
{
    pub deprecated_syscall_handler: Rc<RwLock<DeprecatedOsSyscallHandler<PCS>>>,
}

impl<PCS> DeprecatedOsSyscallHandlerWrapper<PCS>
where
    PCS: PerContractStorage,
{
    pub fn new(exec_wrapper: ExecutionHelperWrapper<PCS>, syscall_ptr: Relocatable, block_info: BlockInfo) -> Self {
        Self {
            deprecated_syscall_handler: Rc::new(RwLock::new(DeprecatedOsSyscallHandler {
                exec_wrapper,
                syscall_ptr,
                block_info,
            })),
        }
    }
}