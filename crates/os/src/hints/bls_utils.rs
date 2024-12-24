use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::Felt252;
use lazy_static::lazy_static;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::Signed;

lazy_static! {
    static ref BASE: BigInt = BigInt::from(2).pow(86);
}

pub fn split(num: Felt252) -> Result<Vec<Felt252>, HintError> {
    let mut a = Vec::with_capacity(3);
    let mut num = num.to_bigint();
    for _ in 0..2 {
        let (q, residue) = num.div_mod_floor(&BASE);
        num = q;
        a.push(residue);
    }
    if num.abs() >= BigInt::from(2).pow(127) {
        return Err(HintError::AssertionFailed(
            "remainder should be less than 2**127"
                .to_string()
                .into_boxed_str(),
        ));
    }
    a.push(num);

    Ok(a.into_iter().map(|big| big.into()).collect())
}
