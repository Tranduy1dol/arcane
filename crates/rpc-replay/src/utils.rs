use starknet_types_core::felt::Felt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeltConversionError {
    #[error("Overflow Error: Felt exceeds u128 max value")]
    OverflowError,
    #[error("{0}")]
    CustomError(String),
}

pub fn felt_to_u128(felt: &Felt) -> Result<u128, FeltConversionError> {
    let digits = felt.to_be_digits();

    // Check if there are any significant bits in the higher 128 bits
    if digits[0] != 0 || digits[1] != 0 {
        return Err(FeltConversionError::OverflowError);
    }

    // Safe conversion since we've checked for overflow
    Ok(((digits[2] as u128) << 64) + digits[3] as u128)
}

/// Executes a coroutine from a synchronous context.
/// Fails if no Tokio runtime is present.
pub(crate) fn execute_coroutine<F, T>(coroutine: F) -> Result<T, tokio::runtime::TryCurrentError>
where
    F: std::future::Future<Output = T>,
{
    let tokio_runtime_handle = tokio::runtime::Handle::try_current()?;
    Ok(tokio::task::block_in_place(|| {
        tokio_runtime_handle.block_on(coroutine)
    }))
}
