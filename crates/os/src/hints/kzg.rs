use crate::execution::helper::ExecutionHelperWrapper;
use crate::hints::vars;
use crate::starknet::starknet_storage::PerContractStorage;
use crate::utils::{execute_coroutine, get_constant};
use c_kzg::{Blob, KzgCommitment, BYTES_PER_FIELD_ELEMENT};
use cairo_vm::hint_processor::builtin_hint_processor::hint_utils::{
    get_integer_from_var_name, get_ptr_from_var_name, insert_value_from_var_name,
};
use cairo_vm::hint_processor::hint_processor_definition::HintReference;
use cairo_vm::serde::deserialize_program::ApTracking;
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::vm::vm_core::VirtualMachine;
use cairo_vm::Felt252;
use indoc::indoc;
use num_bigint::{BigInt, ParseBigIntError};
use num_traits::{Num, One, Zero};
use std::collections::HashMap;
use std::num::ParseIntError;
use std::path::Path;

const FIELD_ELEMENTS_PER_BLOB: usize = 4096;
const COMMITMENT_BYTES_LENGTH: usize = 48;
const BLOB_SUBGROUP_GENERATOR: &str =
    "39033254847818212395286706435128746857159659164139250548781411570340225835782";
const BLS_PRIME: &str =
    "52435875175126190479447740508185965837690552500527637822603658699938581184513";
const COMMITMENT_BITS: usize = 384;
const COMMITMENT_BITS_MIDPOINT: usize = COMMITMENT_BITS / 2;

#[derive(Debug, thiserror::Error)]
pub enum FftError {
    #[error("Group is missing last element")]
    GroupMissingLastElement,

    #[error("Invalid binary cast to usize: {0}")]
    InvalidBinaryToUsize(ParseIntError),

    #[error("Could not parse BigInt: {0}")]
    ParseBigIntError(ParseBigIntError),

    #[error("Encountered a c_kzg error: {0}")]
    CKzgError(#[from] c_kzg::Error),

    #[error("Too many coefficients")]
    TooManyCoefficients,
}

pub const STORE_DA_SEGMENT: &str = indoc! {r#"import itertools

from starkware.python.utils import blockify

kzg_manager.store_da_segment(
    da_segment=memory.get_range_as_ints(addr=ids.state_updates_start, size=ids.da_size)
)
kzg_commitments = [
    kzg_manager.polynomial_coefficients_to_kzg_commitment_callback(chunk)
    for chunk in blockify(kzg_manager.da_segment, chunk_size=ids.BLOB_LENGTH)
]

ids.n_blobs = len(kzg_commitments)
ids.kzg_commitments = segments.add_temp_segment()
ids.evals = segments.add_temp_segment()

segments.write_arg(ids.kzg_commitments.address_, list(itertools.chain(*kzg_commitments)))"#};

pub fn store_da_segment<PCS>(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
) -> Result<(), HintError>
where
    PCS: PerContractStorage + 'static,
{
    execute_coroutine(store_da_segment_async::<PCS>(
        vm,
        exec_scopes,
        ids_data,
        ap_tracking,
        constants,
    ))?
}

pub async fn store_da_segment_async<PCS>(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
) -> Result<(), HintError>
where
    PCS: PerContractStorage + 'static,
{
    let state_updates_start =
        get_ptr_from_var_name(vars::ids::STATE_UPDATES_START, vm, ids_data, ap_tracking)?;
    let da_size =
        get_integer_from_var_name(vars::ids::DA_SIZE, vm, ids_data, ap_tracking)?.to_biguint();
    let da_size: usize = da_size.try_into().map_err(|_| HintError::BigintToU32Fail)?;

    let da_segment: Vec<Felt252> = vm
        .get_integer_range(state_updates_start, da_size)?
        .into_iter()
        .map(|s| *s)
        .collect();

    let blob_length = get_constant(vars::ids::BLOB_LENGTH, constants)?.to_biguint();
    let blob_length: usize = blob_length
        .try_into()
        .map_err(|_| HintError::BigintToU32Fail)?;

    let kzg_commitments: Vec<(Felt252, Felt252)> = da_segment
        .chunks(blob_length)
        .map(|chunk| {
            let coefficients: Vec<BigInt> = chunk.iter().map(|f| f.to_bigint()).collect();
            let res: (BigInt, BigInt) =
                polynomial_coefficients_to_kzg_commitment(coefficients).unwrap(); // TODO: unwrap
            (res.0.into(), res.1.into())
        })
        .collect();

    let ehw = exec_scopes.get::<ExecutionHelperWrapper<PCS>>(vars::scopes::EXECUTION_HELPER)?;
    let kzg_manager = &mut ehw.execution_helper.write().await.kzg_manager;
    kzg_manager.store_da_segment(da_segment)?;

    let n_blobs = kzg_commitments.len();
    let kzg_commitments_segment = vm.add_temporary_segment();
    let evals_segment = vm.add_temporary_segment();

    insert_value_from_var_name(vars::ids::N_BLOBS, n_blobs, vm, ids_data, ap_tracking)?;
    insert_value_from_var_name(
        vars::ids::KZG_COMMITMENTS,
        kzg_commitments_segment,
        vm,
        ids_data,
        ap_tracking,
    )?;
    insert_value_from_var_name(vars::ids::EVALS, evals_segment, vm, ids_data, ap_tracking)?;

    let kzg_commitments_flattened: Vec<MaybeRelocatable> = kzg_commitments
        .into_iter()
        .flat_map(|c| [c.0.into(), c.1.into()])
        .collect();
    vm.write_arg(kzg_commitments_segment, &kzg_commitments_flattened)?;

    Ok(())
}

fn polynomial_coefficients_to_kzg_commitment(
    coefficients: Vec<BigInt>,
) -> Result<(BigInt, BigInt), FftError> {
    let blob = polynomial_coefficients_to_blob(coefficients)?;
    let commitment_bytes =
        blob_to_kzg_commitment(&Blob::from_bytes(&blob).map_err(FftError::CKzgError)?)
            .map_err(FftError::CKzgError)?;
    assert_eq!(
        commitment_bytes.len(),
        COMMITMENT_BYTES_LENGTH,
        "Bad commitment bytes length."
    );
    let kzg_bigint = BigInt::from_str_radix(&commitment_bytes.as_hex_string(), 16)
        .map_err(FftError::ParseBigIntError)?;
    Ok(split_commitment(kzg_bigint))
}

fn polynomial_coefficients_to_blob(coefficients: Vec<BigInt>) -> Result<Vec<u8>, FftError> {
    if coefficients.len() > FIELD_ELEMENTS_PER_BLOB {
        return Err(FftError::TooManyCoefficients);
    }

    // Pad with zeros to complete FIELD_ELEMENTS_PER_BLOB coefficients
    let mut padded_coefficients = coefficients;
    padded_coefficients.resize(FIELD_ELEMENTS_PER_BLOB, BigInt::zero());

    // Perform FFT on the coefficients
    let generator =
        BigInt::from_str_radix(BLOB_SUBGROUP_GENERATOR, 10).map_err(FftError::ParseBigIntError)?;
    let prime = BigInt::from_str_radix(BLS_PRIME, 10).map_err(FftError::ParseBigIntError)?;
    let fft_result = fft(&padded_coefficients, &generator, &prime, true)?;

    // Serialize the FFT result into a blob
    Ok(serialize_blob(&fft_result))
}

pub fn blob_to_kzg_commitment(blob: &Blob) -> Result<KzgCommitment, c_kzg::Error> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("kzg")
        .join("trusted_setup.txt");
    KzgCommitment::blob_to_kzg_commitment(
        blob,
        &c_kzg::KzgSettings::load_trusted_setup_file(&path)?,
    )
}

fn split_commitment(num: BigInt) -> (BigInt, BigInt) {
    // Ensure the input is 384 bits (48 bytes)
    let num = num & &((BigInt::from(1) << COMMITMENT_BITS) - 1);

    // Split the number
    let mask = (BigInt::from(1) << COMMITMENT_BITS_MIDPOINT) - 1;
    let low = &num & &mask;
    let high = &num >> COMMITMENT_BITS_MIDPOINT;

    (low, high)
}

fn fft(
    coeffs: &[BigInt],
    generator: &BigInt,
    prime: &BigInt,
    bit_reversed: bool,
) -> Result<Vec<BigInt>, FftError> {
    if coeffs.is_empty() {
        return Ok(vec![]);
    }

    let coeffs_len = coeffs.len();
    assert!(
        coeffs_len.is_power_of_two(),
        "Length is not a power of two."
    );

    let mut group = vec![BigInt::one()];
    for _ in 0..(coeffs_len - 1) {
        let last = group.last().ok_or(FftError::GroupMissingLastElement)?;
        group.push((last * generator) % prime);
    }

    let mut values = inner_fft(coeffs, &group, prime);

    if bit_reversed {
        // Python equivalent: width = coeffs_len.bit_length() - 1.
        // Since coeffs_len is a power of two, width is set to the position of the last set bit.
        let width = coeffs_len.trailing_zeros() as usize;
        let perm = (0..coeffs_len)
            .map(|i| {
                let binary = format!("{:0width$b}", i, width = width);
                usize::from_str_radix(&binary.chars().rev().collect::<String>(), 2)
                    .map_err(FftError::InvalidBinaryToUsize)
            })
            .collect::<Result<Vec<_>, _>>()?;
        values = perm.into_iter().map(|i| values[i].clone()).collect();
    }

    Ok(values)
}

fn inner_fft(coeffs: &[BigInt], group: &[BigInt], prime: &BigInt) -> Vec<BigInt> {
    if coeffs.len() == 1 {
        return coeffs.to_vec();
    }

    // These calls involve a clone and a collect
    // This not cheap and can possibly be improved by using dynamic iterators or transform the function to make it not recursive
    let f_even = inner_fft(
        &coeffs.iter().step_by(2).cloned().collect::<Vec<_>>(),
        &group.iter().step_by(2).cloned().collect::<Vec<_>>(),
        prime,
    );
    let f_odd = inner_fft(
        &coeffs
            .iter()
            .skip(1)
            .step_by(2)
            .cloned()
            .collect::<Vec<_>>(),
        &group.iter().step_by(2).cloned().collect::<Vec<_>>(),
        prime,
    );

    let group_mul_f_odd: Vec<BigInt> = group
        .iter()
        .take(f_odd.len())
        .zip(f_odd.iter())
        .map(|(g, f)| (g * f) % prime)
        .collect();

    let mut result = Vec::with_capacity(coeffs.len());
    for i in 0..f_even.len() {
        result.push((f_even[i].clone() + &group_mul_f_odd[i]) % prime);
    }
    for i in 0..f_even.len() {
        // Ensure non-negative diff by adding prime to the value before applying the modulo
        let diff = (f_even[i].clone() - &group_mul_f_odd[i] + prime) % prime;
        result.push(diff);
    }

    result
}

fn serialize_blob(blob: &[BigInt]) -> Vec<u8> {
    assert_eq!(blob.len(), FIELD_ELEMENTS_PER_BLOB, "Bad blob size.");
    blob.iter()
        .flat_map(|x| to_bytes(x, BYTES_PER_FIELD_ELEMENT))
        .collect()
}

fn to_bytes(x: &BigInt, length: usize) -> Vec<u8> {
    use std::iter::repeat;
    let mut bytes = x.to_bytes_be().1;
    let padding = length.saturating_sub(bytes.len());
    if padding > 0 {
        let mut padded_bytes = repeat(0u8).take(padding).collect::<Vec<u8>>();
        padded_bytes.extend(bytes);
        bytes = padded_bytes;
    }
    bytes
}
