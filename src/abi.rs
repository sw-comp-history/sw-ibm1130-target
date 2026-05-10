//! Calling convention for the IBM 1130. See `docs/abi.md` for full
//! rationale and citations to the historical IBM manuals.

use sw_ibm1130_isa::{Ibm1130, Reg};
use sw_target_core::CallingConvention;

/// IBM 1130 calling convention. Anchored on the historical CALL /
/// LIBF idioms documented in IBM 1130 Subroutine Library
/// (C26-5929-4); see `docs/abi.md`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Ibm1130CallConv;

const ARG_REGS: &[Reg] = &[Reg::Acc];
const CALLER_SAVED: &[Reg] = &[Reg::Acc, Reg::Ext, Reg::Xr1];
const CALLEE_SAVED: &[Reg] = &[Reg::Xr2];

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

    /// No distinct frame pointer: XR2 is both the frame base and
    /// the logical SP. See `docs/abi.md` Sec 6.
    fn frame_pointer() -> Option<Reg> {
        None
    }

    /// XR2 is the frame base; this ABI does not separate SP from FP.
    fn stack_pointer() -> Reg {
        Reg::Xr2
    }

    const STACK_ALIGNMENT: usize = 1;
}
