use std::rc::Rc;
use blockifier::execution::execution_utils::ReadOnlySegments;
use cairo_vm::types::relocatable::Relocatable;
use tokio::sync::RwLock;
use crate::execution::helper::ExecutionHelperWrapper;
use crate::starknet::starknet_storage::PerContractStorage;

#[derive(Debug)]
pub struct OsSyscallHandler<PCS: PerContractStorage>
where
    PCS: PerContractStorage,
{
    pub exec_wrapper: ExecutionHelperWrapper<PCS>,
    pub syscall_ptr: Option<Relocatable>,
    pub segments: ReadOnlySegments,
}

#[derive(Debug)]
pub struct OsSyscallHandlerWrapper<PCS>
where
    PCS: PerContractStorage,
{
    pub syscall_handler: Rc<RwLock<OsSyscallHandler<PCS>>>,
}

impl<PCS> OsSyscallHandlerWrapper<PCS>
where
    PCS: PerContractStorage + 'static,
{
    pub fn new(exec_wrapper: ExecutionHelperWrapper<PCS>) -> Self {
        Self {
            syscall_handler: Rc::new(RwLock::new(OsSyscallHandler {
                exec_wrapper,
                syscall_ptr: None,
                segments: ReadOnlySegments::default(),
            })),
        }
    }
}