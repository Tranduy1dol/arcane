use cairo_vm::Felt252;
use starknet_api::core::ChainId;
use starknet_core::types::Felt;

/// Converts a ChainId object into a felt.
pub fn chain_id_to_felt(chain_id: &ChainId) -> Felt252 {
    Felt252::from_bytes_be_slice(chain_id.to_string().as_bytes())
}

/// Builds a ChainId from a felt.
/// This function reads the felt as ASCII bytes. Leading zeroes are skipped.
pub fn chain_id_from_felt(felt: Felt) -> ChainId {
    // Skip leading zeroes
    let chain_id_bytes: Vec<_> = felt.to_bytes_be().into_iter().skip_while(|byte| *byte == 0u8).collect();
    let chain_id_str = String::from_utf8_lossy(&chain_id_bytes);
    ChainId::from(chain_id_str.into_owned())
}
