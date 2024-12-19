use starknet::core::types::Felt;
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