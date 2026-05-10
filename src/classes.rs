//! Register classes for the IBM 1130 allocator. See `docs/abi.md`
//! Sec 2 for the full role assignment and citations.
//!
//! GPR = freely allocatable individual 16-bit registers (ACC, XR1).
//!
//! Reserved = registers the allocator must not touch:
//!
//! - EXT (used only as the low half of the ACC+EXT pair)
//! - XR2 (frame base; doubles as SP per docs/abi.md Sec 6)
//! - XR3 (LIBF transfer-vector base; loader-managed program-lifetime
//!   invariant -- never modified by user code)
//! - IAR (program counter)
//!
//! Fixed pairs = register pairs allocated together: (ACC, EXT) for
//! 32-bit M / D / LDD / STD / AD / SD.

use sw_ibm1130_isa::{Ibm1130, Reg};
use sw_target_core::RegisterClasses;

/// IBM 1130 register classes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Ibm1130RegClasses;

const GPR: &[Reg] = &[Reg::Acc, Reg::Xr1];
const RESERVED: &[Reg] = &[Reg::Ext, Reg::Xr2, Reg::Xr3, Reg::Iar];
const FIXED_PAIRS: &[(Reg, Reg)] = &[(Reg::Acc, Reg::Ext)];

impl RegisterClasses<Ibm1130> for Ibm1130RegClasses {
    fn gpr() -> &'static [Reg] {
        GPR
    }

    fn reserved() -> &'static [Reg] {
        RESERVED
    }

    fn fixed_pairs() -> &'static [(Reg, Reg)] {
        FIXED_PAIRS
    }
}
