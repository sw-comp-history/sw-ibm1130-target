//! Calling convention for the IBM 1130. See `docs/abi.md` for rationale.

use sw_ibm1130_isa::{Ibm1130, Reg};
use sw_target_core::CallingConvention;

/// IBM 1130 calling convention (invented; see `docs/abi.md`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Ibm1130CallConv;

const ARG_REGS: &[Reg] = &[Reg::Acc];
const CALLER_SAVED: &[Reg] = &[Reg::Acc, Reg::Ext, Reg::Xr1];
const CALLEE_SAVED: &[Reg] = &[Reg::Xr2, Reg::Xr3];

impl CallingConvention<Ibm1130> for Ibm1130CallConv {
    fn arg_regs() -> &'static [Reg] {
        ARG_REGS
    }

    fn return_reg() -> Reg {
        Reg::Acc
    }

    fn caller_saved() -> &'static [Reg] {
        CALLER_SAVED
    }

    fn callee_saved() -> &'static [Reg] {
        CALLEE_SAVED
    }

    fn frame_pointer() -> Option<Reg> {
        Some(Reg::Xr3)
    }

    fn stack_pointer() -> Reg {
        Reg::Xr2
    }

    const STACK_ALIGNMENT: usize = 1;
}
