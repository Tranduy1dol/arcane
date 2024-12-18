use std::collections::{HashMap, HashSet};
use cairo_vm::Felt252;
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::types::relocatable::Relocatable;
use cairo_vm::vm::errors::hint_errors::HintError;
use crate::cairo_types::builtins::{HashBuiltin, SpongeHashBuiltin};
use crate::hints::vars;
use crate::utils::get_variable_from_root_exec_scope;

pub type Preimage = HashMap<Felt252, Vec<Felt252>>;

#[derive(Clone, Debug, PartialEq)]
pub struct PatriciaSkipValidationRunner {
    pub verified_addresses: HashSet<Relocatable>,
}

#[derive(Clone, Debug)]
pub enum PatriciaTreeMode {
    Class,
    State,
}

pub fn get_hash_builtin_fields(exec_scopes: &ExecutionScopes) -> Result<(usize, usize, usize), HintError> {
    let patricia_tree_mode: PatriciaTreeMode =
        get_variable_from_root_exec_scope(exec_scopes, vars::scopes::PATRICIA_TREE_MODE)?;
    log::trace!("Patricia tree mode: {patricia_tree_mode:?}");
    Ok(match patricia_tree_mode {
        PatriciaTreeMode::Class => {
            (SpongeHashBuiltin::x_offset(), SpongeHashBuiltin::y_offset(), SpongeHashBuiltin::result_offset())
        }
        PatriciaTreeMode::State => (HashBuiltin::x_offset(), HashBuiltin::y_offset(), HashBuiltin::result_offset()),
    })
}

pub fn skip_verification_if_configured(
    exec_scopes: &mut ExecutionScopes,
    address: Relocatable,
) -> Result<(), HintError> {
    let patricia_skip_validation_runner: &mut Option<PatriciaSkipValidationRunner> =
        exec_scopes.get_mut_ref(vars::scopes::PATRICIA_SKIP_VALIDATION_RUNNER)?;
    if let Some(skipped) = patricia_skip_validation_runner {
        skipped.verified_addresses.insert(address);
    }

    Ok(())
}
