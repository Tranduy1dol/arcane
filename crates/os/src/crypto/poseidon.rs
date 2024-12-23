use starknet_crypto::{poseidon_hash, FieldElement};
use arcane_os_type::hash::Hash;
use crate::storage::storage::HashFunctionType;

#[derive(Clone, Debug, PartialEq)]
pub struct PoseidonHash;

impl HashFunctionType for PoseidonHash {
    fn hash(x: &[u8], y: &[u8]) -> Hash {
        let x_felt = FieldElement::from_byte_slice_be(x).unwrap();
        let y_felt = FieldElement::from_byte_slice_be(y).unwrap();

        Hash::from_bytes_be(poseidon_hash(x_felt, y_felt).to_bytes_be())
    }
}