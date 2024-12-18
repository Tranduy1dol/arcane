use std::collections::HashMap;
use std::ops::Sub;
use cairo_vm::Felt252;
use cairo_vm::types::errors::math_errors::MathError;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

pub type TreeIndex = BigUint;

#[derive(Debug, Copy, Clone, PartialEq, Default, Eq, Hash, Serialize, Deserialize)]
pub struct Height(pub u64);

#[derive(Debug, Clone, PartialEq, Default, Eq, Hash)]
pub struct NodePath(pub BigUint);

#[derive(Debug, Copy, Clone, PartialEq, Default, Eq)]
pub struct Length(pub u64);

#[derive(Debug, Clone, PartialEq, Default, Eq, Hash)]
pub struct DescentStart(pub Height, pub NodePath);

#[derive(Debug, Clone, PartialEq, Default, Eq)]
pub struct DescentPath(pub Length, pub NodePath);
pub type DescentMap = HashMap<DescentStart, DescentPath>;

impl TryFrom<Felt252> for Height {
    type Error = MathError;

    fn try_from(value: Felt252) -> Result<Self, Self::Error> {
        let height = value.to_u64().ok_or(MathError::Felt252ToU64Conversion(Box::new(value)))?;
        Ok(Self(height))
    }
}

impl Sub<u64> for Height {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}