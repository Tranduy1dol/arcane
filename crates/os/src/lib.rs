#![feature(once_cell_try)]

pub mod cairo_types;
pub mod config;
pub mod crypto;
pub mod error;
pub mod execution;
pub mod hints;
pub mod io;
pub mod starknet;
pub mod starkware_utils;
pub mod storage;
pub mod r#type;
pub mod utils;

use crate::error::ArcaneError;
use crate::execution::deprecated_syscall_handler::DeprecatedOsSyscallHandlerWrapper;
use crate::execution::helper::ExecutionHelperWrapper;
use crate::execution::syscall_handler::OsSyscallHandlerWrapper;
use crate::hints::types::{PatriciaSkipValidationRunner, PatriciaTreeMode};
use crate::hints::vars;
use crate::io::input::StarknetOsInput;
use crate::io::output::StarknetOsOutput;
use crate::starknet::starknet_storage::PerContractStorage;
use blockifier::context::BlockContext;
use cairo_vm::cairo_run::CairoRunConfig;
use cairo_vm::types::program::Program;
use cairo_vm::vm::errors::vm_exception::VmException;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use cairo_vm::vm::runners::cairo_runner::CairoRunner;
use cairo_vm::vm::vm_core::VirtualMachine;
use std::rc::Rc;

pub fn run_os<PCS>(
    complied_os: &[u8],
    layout: &str,
    os_input: Rc<StarknetOsInput>,
    block_context: BlockContext,
    execution_helper: ExecutionHelperWrapper<PCS>,
) -> anyhow::Result<(CairoPie, StarknetOsOutput), ArcaneError>
where
    PCS: PerContractStorage + 'static,
{
    let cairo_run_config = CairoRunConfig {
        layout,
        relocate_mem: true,
        trace_enabled: true,
        ..Default::default()
    };
    let allow_missing_builtin = cairo_run_config.allow_missing_builtins.unwrap_or(false);

    let os_program = Program::from_bytes(complied_os, Some(cairo_run_config.entrypoint))
        .map_err(|e| ArcaneError::Runner(e.into()))?;
    let mut cairo_runner = CairoRunner::new(
        &os_program,
        cairo_run_config.layout,
        cairo_run_config.proof_mode,
    )
    .map_err(|e| ArcaneError::Runner(e.into()))?;
    let mut virtual_machine = VirtualMachine::new(cairo_run_config.trace_enabled);
    let end = cairo_runner
        .initialize(&mut virtual_machine, allow_missing_builtin)
        .map_err(|e| ArcaneError::Runner(e.into()))?;

    let deprecated_syscall_handler = DeprecatedOsSyscallHandlerWrapper::new(
        execution_helper.clone(),
        virtual_machine.add_memory_segment(),
        block_context.block_info().clone(),
    );

    let syscall_handler = OsSyscallHandlerWrapper::new(execution_helper.clone());

    cairo_runner
        .exec_scopes
        .insert_value(vars::scopes::OS_INPUT, os_input);
    cairo_runner
        .exec_scopes
        .insert_box(vars::scopes::BLOCK_CONTEXT, Box::new(block_context));
    cairo_runner
        .exec_scopes
        .insert_value(vars::scopes::EXECUTION_HELPER, execution_helper);
    cairo_runner.exec_scopes.insert_value(
        vars::scopes::DEPRECATED_SYSCALL_HANDLER,
        deprecated_syscall_handler,
    );
    cairo_runner
        .exec_scopes
        .insert_value(vars::scopes::SYSCALL_HANDLER, syscall_handler);
    cairo_runner.exec_scopes.insert_value(
        vars::scopes::PATRICIA_SKIP_VALIDATION_RUNNER,
        None::<PatriciaSkipValidationRunner>,
    );
    cairo_runner
        .exec_scopes
        .insert_value(vars::scopes::PATRICIA_TREE_MODE, PatriciaTreeMode::State);
    cairo_runner
        .exec_scopes
        .insert_value::<Option<usize>>(vars::scopes::FIND_ELEMENT_MAX_SIZE, None);

    let mut sn_hint_processor = hints::SnosHintProcessor::<PCS>::default();
    cairo_runner
        .run_until_pc(end, &mut virtual_machine, &mut sn_hint_processor)
        .map_err(|e| VmException::from_vm_error(&cairo_runner, &virtual_machine, e))
        .map_err(|err| ArcaneError::Runner(err.into()))?;

    cairo_runner
        .end_run(
            cairo_run_config.disable_trace_padding,
            false,
            &mut virtual_machine,
            &mut sn_hint_processor,
        )
        .map_err(|e| ArcaneError::Runner(e.into()))?;

    let os_output = StarknetOsOutput::from_run(&virtual_machine)?;

    virtual_machine
        .verify_auto_deductions()
        .map_err(|e| ArcaneError::Runner(e.into()))?;
    cairo_runner
        .read_return_values(&mut virtual_machine)
        .map_err(|e| ArcaneError::Runner(e.into()))?;
    cairo_runner
        .relocate(&mut virtual_machine, cairo_run_config.relocate_mem)
        .map_err(|e| ArcaneError::Runner(e.into()))?;

    let pie = cairo_runner
        .get_cairo_pie(&virtual_machine)
        .map_err(|e| ArcaneError::PieParsing(format!("{e}")))?;

    Ok((pie, os_output))
}
