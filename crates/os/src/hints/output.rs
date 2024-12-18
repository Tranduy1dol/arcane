use std::cmp::min;
use std::collections::HashMap;
use cairo_vm::Felt252;
use cairo_vm::hint_processor::builtin_hint_processor::hint_utils::{get_integer_from_var_name, get_ptr_from_var_name, insert_value_from_var_name};
use cairo_vm::hint_processor::hint_processor_definition::HintReference;
use cairo_vm::serde::deserialize_program::ApTracking;
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::vm::vm_core::VirtualMachine;
use indoc::indoc;
use num_integer::div_ceil;
use crate::hints::vars;
use crate::utils::get_variable_from_root_exec_scope;

const MAX_PAGE_SIZE: usize = 3800;

#[rustfmt::skip]
pub const SET_TREE_STRUCTURE: &str = indoc! {r#"from starkware.python.math_utils import div_ceil

if __serialize_data_availability_create_pages__:
    onchain_data_start = ids.da_start
    onchain_data_size = ids.output_ptr - onchain_data_start

    max_page_size = 3800
    n_pages = div_ceil(onchain_data_size, max_page_size)
    for i in range(n_pages):
        start_offset = i * max_page_size
        output_builtin.add_page(
            page_id=1 + i,
            page_start=onchain_data_start + start_offset,
            page_size=min(onchain_data_size - start_offset, max_page_size),
        )
    # Set the tree structure to a root with two children:
    # * A leaf which represents the main part
    # * An inner node for the onchain data part (which contains n_pages children).
    #
    # This is encoded using the following sequence:
    output_builtin.add_attribute('gps_fact_topology', [
        # Push 1 + n_pages pages (all of the pages).
        1 + n_pages,
        # Create a parent node for the last n_pages.
        n_pages,
        # Don't push additional pages.
        0,
        # Take the first page (the main part) and the node that was created (onchain data)
        # and use them to construct the root of the fact tree.
        2,
    ])"#};

pub fn set_tree_structure(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    _constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let serialize_data_availability_create_pages: bool =
        get_variable_from_root_exec_scope(exec_scopes, vars::scopes::SERIALIZE_DATA_AVAILABILITY_CREATE_PAGES)?;

    if !serialize_data_availability_create_pages {
        return Ok(());
    }

    let onchain_data_start = get_ptr_from_var_name(vars::ids::DA_START, vm, ids_data, ap_tracking)?;
    let output_ptr = get_ptr_from_var_name(vars::ids::OUTPUT_PTR, vm, ids_data, ap_tracking)?;
    let onchain_data_size = (output_ptr - onchain_data_start)?;

    let output_builtin = vm.get_output_builtin_mut()?;

    let n_pages = div_ceil(onchain_data_size, MAX_PAGE_SIZE);
    for i in 0..n_pages {
        let start_offset = i * MAX_PAGE_SIZE;
        let page_id = i + 1;
        let page_start = (onchain_data_start + start_offset)?;
        let page_size = min(onchain_data_size - start_offset, MAX_PAGE_SIZE);
        output_builtin
            .add_page(page_id, page_start, page_size)
            .map_err(|e| HintError::CustomHint(e.to_string().into_boxed_str()))?;
    }

    // Set the tree structure to a root with two children:
    // * A leaf which represents the main part
    // * An inner node for the onchain data part (which contains n_pages children).
    //
    // This is encoded using the following sequence:
    output_builtin.add_attribute(
        "gps_fact_topology".to_string(),
        vec![
            // Push 1 + n_pages pages (all of the pages).
            1 + n_pages,
            // Create a parent node for the last n_pages.
            n_pages,
            // Don't push additional pages.
            0,
            // Take the first page (the main part) and the node that was created (onchain data)
            // and use them to construct the root of the fact tree.
            2,
        ],
    );

    Ok(())
}

pub const SET_STATE_UPDATES_START: &str = indoc! {r#"if ids.use_kzg_da:
    ids.state_updates_start = segments.add()
else:
    # Assign a temporary segment, to be relocated into the output segment.
    ids.state_updates_start = segments.add_temp_segment()"#};

pub fn set_state_updates_start(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    _constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let use_kzg_da_felt = get_integer_from_var_name(vars::ids::USE_KZG_DA, vm, ids_data, ap_tracking)?;

    let use_kzg_da = if use_kzg_da_felt == Felt252::ONE {
        Ok(true)
    } else if use_kzg_da_felt == Felt252::ZERO {
        Ok(false)
    } else {
        Err(HintError::CustomHint("ids.use_kzg_da is not a boolean".to_string().into_boxed_str()))
    }?;

    if use_kzg_da {
        insert_value_from_var_name(vars::ids::STATE_UPDATES_START, vm.add_memory_segment(), vm, ids_data, ap_tracking)?;
    } else {
        // Assign a temporary segment, to be relocated into the output segment.
        insert_value_from_var_name(
            vars::ids::STATE_UPDATES_START,
            vm.add_temporary_segment(),
            vm,
            ids_data,
            ap_tracking,
        )?;
    }

    Ok(())
}