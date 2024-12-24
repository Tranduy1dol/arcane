use std::collections::HashMap;
use cairo_vm::Felt252;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::vm::runners::builtin_runner::BuiltinRunner;
use cairo_vm::vm::vm_core::VirtualMachine;
use num_traits::{ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use crate::error::ArcaneError;

const PREVIOUS_MERKLE_UPDATE_OFFSET: usize = 0;
const NEW_MERKLE_UPDATE_OFFSET: usize = 1;
const PREV_BLOCK_NUMBER_OFFSET: usize = 2;
const NEW_BLOCK_NUMBER_OFFSET: usize = 3;
const PREV_BLOCK_HASH_OFFSET: usize = 4;
const NEW_BLOCK_HASH_OFFSET: usize = 5;
const OS_PROGRAM_HASH_OFFSET: usize = 6;
const CONFIG_HASH_OFFSET: usize = 7;
const USE_KZG_DA_OFFSET: usize = 8;
const FULL_OUTPUT_OFFSET: usize = 9;
const HEADER_SIZE: usize = 10;
const KZG_N_BLOBS_OFFSET: usize = 1;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ContractChanges {
    pub addr: Felt252,
    pub nonce: Felt252,
    pub class_hash: Option<Felt252>,
    pub storage_changes: HashMap<Felt252, Felt252>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StarknetOsOutput {
    pub initial_root: Felt252,
    pub final_root: Felt252,
    pub prev_block_number: Felt252,
    pub new_block_number: Felt252,
    pub prev_block_hash: Felt252,
    pub new_block_hash: Felt252,
    pub os_program_hash: Felt252,
    pub starknet_os_config_hash: Felt252,
    pub use_kzg_da: Felt252,
    pub full_output: Felt252,
    pub messages_to_l1: Vec<Felt252>,
    pub messages_to_l2: Vec<Felt252>,
    pub contracts: Vec<ContractChanges>,
    pub classes: HashMap<Felt252, Felt252>,
}

impl StarknetOsOutput {
    pub fn from_run(vm: &VirtualMachine) -> Result<Self, ArcaneError> {
        let (output_base, output_size) = get_output_info(vm)?;
        let raw_output = get_raw_output(vm, output_base, output_size)?;
        deserialize_os_output(&mut raw_output.into_iter())
    }
}

fn get_output_info(vm: &VirtualMachine) -> Result<(usize, usize), ArcaneError> {
    let n_builtins = vm.get_builtin_runners().len();
    let builtin_end_ptrs = vm.get_return_values(n_builtins).map_err(|e| ArcaneError::CatchAll(e.to_string()))?;
    let output_base = vm
        .get_builtin_runners()
        .iter()
        .find(|&elt| matches!(elt, BuiltinRunner::Output(_)))
        .expect("Os vm should have the output builtin")
        .base();

    let output_size = match builtin_end_ptrs[0] {
        MaybeRelocatable::Int(_) => {
            return Err(ArcaneError::CatchAll("expected a relocatable as output builtin end pointer".to_string()));
        }
        MaybeRelocatable::RelocatableValue(address) => {
            if address.segment_index as usize != output_base {
                return Err(ArcaneError::CatchAll(format!(
                    "output builtin end pointer ({address}) is not on the expected segment ({output_base})"
                )));
            }
            address.offset
        }
    };

    Ok((output_base, output_size))
}

fn get_raw_output(vm: &VirtualMachine, output_base: usize, output_size: usize) -> Result<Vec<Felt252>, ArcaneError> {
    // Get output and check that everything is an integer.
    let raw_output = vm.get_range((output_base as isize, 0).into(), output_size);
    let raw_output: Result<Vec<Felt252>, _> = raw_output
        .iter()
        .map(|x| {
            if let MaybeRelocatable::Int(val) = x.clone().unwrap().into_owned() {
                Ok(val)
            } else {
                Err(ArcaneError::CatchAll("Output should be all integers".to_string()))
            }
        })
        .collect();

    raw_output
}

fn read_segment<I: Iterator<Item = Felt252>>(
    output_iter: &mut I,
    length: usize,
    item_name: &str,
) -> Result<Vec<Felt252>, ArcaneError> {
    let segment = output_iter.by_ref().take(length).collect::<Vec<_>>();
    if segment.len() != length {
        return Err(ArcaneError::CatchAll(format!(
            "Expected {} {}, could only read {}",
            length,
            item_name,
            segment.len()
        )));
    }
    Ok(segment)
}

pub fn deserialize_os_output<I>(output_iter: &mut I) -> Result<StarknetOsOutput, ArcaneError>
where
    I: Iterator<Item = Felt252>,
{
    let header = read_segment(output_iter, HEADER_SIZE, "header elements")?;
    let use_kzg_da = header[USE_KZG_DA_OFFSET];
    let full_output = header[FULL_OUTPUT_OFFSET];

    if !use_kzg_da.is_zero() {
        let kzg_segment: Vec<_> = output_iter.by_ref().take(2).collect();
        let n_blobs: usize = kzg_segment
            .get(KZG_N_BLOBS_OFFSET)
            .expect("Should have n_blobs in header when using kzg da")
            .to_biguint()
            .try_into()
            .expect("n_blobs should fit in a usize");
        let _: Vec<_> = output_iter.by_ref().take(2 * 2 * n_blobs).collect();
    }

    let (messages_to_l1, messages_to_l2) = deserialize_messages(output_iter)?;

    let (contract_changes, classes) = if use_kzg_da.is_zero() {
        (
            deserialize_contract_state(output_iter, full_output)?,
            deserialize_contract_class_da_changes(output_iter, full_output)?,
        )
    } else {
        (vec![], HashMap::default())
    };

    Ok(StarknetOsOutput {
        initial_root: header[PREVIOUS_MERKLE_UPDATE_OFFSET],
        final_root: header[NEW_MERKLE_UPDATE_OFFSET],
        prev_block_number: header[PREV_BLOCK_NUMBER_OFFSET],
        new_block_number: header[NEW_BLOCK_NUMBER_OFFSET],
        prev_block_hash: header[PREV_BLOCK_HASH_OFFSET],
        new_block_hash: header[NEW_BLOCK_HASH_OFFSET],
        os_program_hash: header[OS_PROGRAM_HASH_OFFSET],
        starknet_os_config_hash: header[CONFIG_HASH_OFFSET],
        use_kzg_da,
        full_output,
        messages_to_l1,
        messages_to_l2,
        contracts: contract_changes,
        classes,
    })
}

fn deserialize_contract_class_da_changes<I: Iterator<Item = Felt252>>(
    output_iter: &mut I,
    full_output: Felt252,
) -> Result<HashMap<Felt252, Felt252>, ArcaneError> {
    let n_actual_updates = next_as_usize(output_iter, "n_actual_updates")?;

    let mut classes = HashMap::with_capacity(n_actual_updates);

    for i in 0..n_actual_updates {
        let class_hash = next_or_fail(output_iter, &format!("class hash #{i}"))?;
        if !full_output.is_zero() {
            next_or_fail(output_iter, &format!("previous compiled class hash #{i}"))?;
        }
        let compiled_class_hash = next_or_fail(output_iter, &format!("compiled class hash #{i}"))?;
        classes.insert(class_hash, compiled_class_hash);
    }

    Ok(classes)
}

fn deserialize_contract_state<I: Iterator<Item = Felt252>>(
    output_iter: &mut I,
    full_output: Felt252,
) -> Result<Vec<ContractChanges>, ArcaneError> {
    let output_n_updates = next_as_usize(output_iter, "output_n_updates")?;
    let mut contract_changes = Vec::with_capacity(output_n_updates);

    for _ in 0..output_n_updates {
        contract_changes.push(deserialize_contract_state_inner(output_iter, full_output)?)
    }

    Ok(contract_changes)
}

fn deserialize_contract_state_inner<I: Iterator<Item = Felt252>>(
    output_iter: &mut I,
    full_output: Felt252,
) -> Result<ContractChanges, ArcaneError> {
    let bound =
        Felt252::from(1u128 << 64).try_into().expect("2**64 should be considered non-zero. Did you change the value?");

    let addr = next_or_fail(output_iter, "contract change addr")?;

    let value = next_or_fail(output_iter, "contract change value")?;
    let (value, n_actual_updates) = value.div_rem(&bound);
    let (was_class_updated, new_state_nonce) = value.div_rem(&bound);

    #[allow(clippy::collapsible_else_if)] // Mirror the Cairo code as much as possible
    let new_state_class_hash = if !full_output.is_zero() {
        next_or_fail(output_iter, "contract change prev_state.class_hash")?;
        Some(next_or_fail(output_iter, "contract change new_state.class_hash")?)
    } else {
        if !was_class_updated.is_zero() {
            Some(next_or_fail(output_iter, "contract change new_state.class_hash")?)
        } else {
            None
        }
    };

    let n_actual_updates = n_actual_updates
        .to_usize()
        .expect("n_updates should be 64-bit by definition. Did you modify the parsing above?");
    let storage_changes = deserialize_da_changes(output_iter, n_actual_updates, full_output)?;

    Ok(ContractChanges { addr, nonce: new_state_nonce, class_hash: new_state_class_hash, storage_changes })
}

fn deserialize_messages<I>(output_iter: &mut I) -> Result<(Vec<Felt252>, Vec<Felt252>), ArcaneError>
where
    I: Iterator<Item = Felt252>,
{
    /// Reads a section with a variable length from the iterator.
    /// Some sections start with a length field N followed by N items.
    fn read_variable_length_segment<I: Iterator<Item = Felt252>>(
        output_iter: &mut I,
        item_name: &str,
    ) -> Result<Vec<Felt252>, ArcaneError> {
        let n_items = next_as_usize(output_iter, item_name)?;
        read_segment(output_iter, n_items, item_name)
    }

    let messages_to_l1 = read_variable_length_segment(output_iter, "L1 messages")?;
    let messages_to_l2 = read_variable_length_segment(output_iter, "L2 messages")?;
    Ok((messages_to_l1, messages_to_l2))
}

fn deserialize_da_changes<I: Iterator<Item = Felt252>>(
    output_iter: &mut I,
    n_updates: usize,
    full_output: Felt252,
) -> Result<HashMap<Felt252, Felt252>, ArcaneError> {
    let mut storage_changes = HashMap::with_capacity(n_updates);

    for i in 0..n_updates {
        let key = next_or_fail(output_iter, &format!("contract change key #{i}"))?;
        if !full_output.is_zero() {
            next_or_fail(output_iter, &format!("contract change prev_value #{i}"))?;
        }
        let new_value = next_or_fail(output_iter, &format!("contract change new_value #{i}"))?;
        storage_changes.insert(key, new_value);
    }

    Ok(storage_changes)
}

fn next_as_usize<I: Iterator<Item = Felt252>>(output_iter: &mut I, item_name: &str) -> Result<usize, ArcaneError> {
    output_iter
        .next()
        .ok_or(ArcaneError::CatchAll(format!("Could not read {item_name} segment size")))?
        .to_usize()
        .ok_or(ArcaneError::CatchAll(format!("{item_name} segment size is too large")))
}

fn next_or_fail<T, I: Iterator<Item = T>>(output_iter: &mut I, item_name: &str) -> Result<T, ArcaneError> {
    output_iter.next().ok_or(ArcaneError::CatchAll(format!("Could not read {item_name} field")))
}