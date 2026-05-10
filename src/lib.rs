//! `sw-ibm1130-target`: IBM 1130 compiler-target description.
//!
//! Implements the `sw_target_core` traits for the IBM 1130: ABI,
//! calling convention, register classes, type widths.
//!
//! The ABI itself is invented for this toolchain; see `docs/abi.md`
//! in this crate for the full specification and rationale.

pub mod abi;
pub mod classes;
pub mod types;

pub use abi::Ibm1130CallConv;
pub use classes::Ibm1130RegClasses;

use sw_target_core::{PrimType, StackDirection, Target};

/// Marker type implementing `sw_target_core::Target` for the IBM 1130.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Ibm1130Target;

impl Target for Ibm1130Target {
    type Arch = sw_ibm1130_isa::Ibm1130;
    type CallConv = Ibm1130CallConv;

    fn type_width(ty: PrimType) -> usize {
        types::type_width(ty)
    }

    fn type_alignment(ty: PrimType) -> usize {
        types::type_alignment(ty)
    }

    const STACK_GROWS: StackDirection = StackDirection::Down;
    const POINTER_BITS: u32 = 16;
}
