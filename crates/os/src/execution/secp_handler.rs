use std::cell::OnceCell;
use ark_ec::short_weierstrass::SWCurveConfig;
use blockifier::execution::syscalls::secp::SecpHintProcessor;
use cairo_vm::types::relocatable::Relocatable;

#[derive(Debug, Default)]
pub struct SecpSyscallProcessor<C: SWCurveConfig> {
    processor: SecpHintProcessor<C>,
    segment: OnceCell<Relocatable>,
}