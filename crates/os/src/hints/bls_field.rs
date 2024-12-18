use std::collections::HashMap;
use cairo_vm::Felt252;
use cairo_vm::hint_processor::builtin_hint_processor::hint_utils::{get_relocatable_from_var_name, insert_value_from_var_name};
use cairo_vm::hint_processor::hint_processor_definition::HintReference;
use cairo_vm::serde::deserialize_program::ApTracking;
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::vm::vm_core::VirtualMachine;
use indoc::indoc;
use num_bigint::BigUint;
use crate::cairo_types::bigint::BigInt3;
use crate::hints::vars;
use crate::utils::get_constant;

pub const COMPUTE_IDS_LOW: &str = indoc! {r#"
    ids.low = (ids.value.d0 + ids.value.d1 * ids.BASE) & ((1 << 128) - 1)"#
};

/// From the Cairo code, we can make the current assumptions:
///
/// * The limbs of value are in the range [0, BASE * 3).
/// * value is in the range [0, 2 ** 256).
pub fn compute_ids_low(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let value_ptr = get_relocatable_from_var_name(vars::ids::VALUE, vm, ids_data, ap_tracking)?;
    let d0 = vm.get_integer((value_ptr + BigInt3::d0_offset())?)?;
    let d1 = vm.get_integer((value_ptr + BigInt3::d1_offset())?)?;

    let base = get_constant(vars::constants::BASE, constants)?;

    let mask = (BigUint::from(1u64) << 128) - BigUint::from(1u64);
    let low = (d0.as_ref() + d1.as_ref() * base).to_biguint() & mask;

    insert_value_from_var_name(vars::ids::LOW, Felt252::from(low), vm, ids_data, ap_tracking)?;

    Ok(())
}
