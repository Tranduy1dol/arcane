use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::{any_box, Felt252};
use std::collections::HashMap;
use tokio::task;

pub(crate) fn custom_hint_error<S: Into<String>>(error: S) -> HintError {
    HintError::CustomHint(error.into().into_boxed_str())
}

pub fn execute_coroutine<F, T>(coroutine: F) -> Result<T, HintError>
where
    F: std::future::Future<Output = T>,
{
    let tokio_runtime_handle = get_tokio_runtime_handle()?;
    Ok(task::block_in_place(|| {
        tokio_runtime_handle.block_on(coroutine)
    }))
}

fn get_tokio_runtime_handle() -> Result<tokio::runtime::Handle, HintError> {
    tokio::runtime::Handle::try_current().map_err(|e| {
        HintError::CustomHint(format!("Tokio runtime not found: {e}").into_boxed_str())
    })
}

pub(crate) fn get_constant<'a>(
    identifier: &'static str,
    constants: &'a HashMap<String, Felt252>,
) -> Result<&'a Felt252, HintError> {
    constants
        .get(identifier)
        .ok_or(HintError::MissingConstant(Box::new(identifier)))
}

pub(crate) fn get_variable_from_root_exec_scope<T>(
    exec_scopes: &ExecutionScopes,
    name: &str,
) -> Result<T, HintError>
where
    T: Clone + 'static,
{
    exec_scopes.data[0]
        .get(name)
        .and_then(|var| var.downcast_ref::<T>().cloned())
        .ok_or(HintError::VariableNotInScopeError(
            name.to_string().into_boxed_str(),
        ))
}

pub(crate) fn set_variable_in_root_exec_scope<T>(
    exec_scopes: &mut ExecutionScopes,
    name: &str,
    value: T,
) where
    T: Clone + 'static,
{
    exec_scopes.data[0].insert(name.to_string(), any_box!(value));
}
