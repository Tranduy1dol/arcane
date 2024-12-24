use crate::storage::storage::{Fact, HashFunctionType, Storage};
use num_bigint::BigUint;

pub trait InnerNodeFact<S, H>: Fact<S, H>
where
    S: Storage,
    H: HashFunctionType,
{
    fn to_tuple(&self) -> Vec<BigUint>;
}
