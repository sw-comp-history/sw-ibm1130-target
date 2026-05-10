//! Smoke tests for the IBM 1130 target trait impls.
//!
//! These are sanity checks against `docs/abi.md`. The detailed ABI
//! rationale lives in that document; these tests pin the encoded
//! decisions to the trait surface.

use sw_ibm1130_isa::Reg;
use sw_ibm1130_target::{Ibm1130CallConv, Ibm1130RegClasses, Ibm1130Target};
use sw_target_core::{CallingConvention, PrimType, RegisterClasses, StackDirection, Target};

#[test]
fn target_constants() {
    assert_eq!(Ibm1130Target::POINTER_BITS, 16);
    assert_eq!(Ibm1130Target::STACK_GROWS, StackDirection::Down);
}

#[test]
fn type_widths_match_abi_table() {
    assert_eq!(Ibm1130Target::type_width(PrimType::I8), 1);
    assert_eq!(Ibm1130Target::type_width(PrimType::U8), 1);
    assert_eq!(Ibm1130Target::type_width(PrimType::Bool), 1);
    assert_eq!(Ibm1130Target::type_width(PrimType::I16), 1);
    assert_eq!(Ibm1130Target::type_width(PrimType::U16), 1);
    assert_eq!(Ibm1130Target::type_width(PrimType::Ptr), 1);
    assert_eq!(Ibm1130Target::type_width(PrimType::I32), 2);
    assert_eq!(Ibm1130Target::type_width(PrimType::U32), 2);
    assert_eq!(Ibm1130Target::type_width(PrimType::I64), 4);
    assert_eq!(Ibm1130Target::type_width(PrimType::U64), 4);
}

#[test]
fn type_alignment_is_one_word_for_all() {
    for ty in [
        PrimType::I8,
        PrimType::U8,
        PrimType::Bool,
        PrimType::I16,
        PrimType::U16,
        PrimType::Ptr,
        PrimType::I32,
        PrimType::U32,
        PrimType::I64,
        PrimType::U64,
    ] {
        assert_eq!(Ibm1130Target::type_alignment(ty), 1);
    }
}

#[test]
fn calling_convention_arg_regs_is_acc_only() {
    assert_eq!(Ibm1130CallConv::arg_regs(), &[Reg::Acc]);
}

#[test]
fn calling_convention_return_reg_is_acc() {
    assert_eq!(Ibm1130CallConv::return_reg(), Reg::Acc);
}

#[test]
fn calling_convention_saved_sets() {
    assert_eq!(
        Ibm1130CallConv::caller_saved(),
        &[Reg::Acc, Reg::Ext, Reg::Xr1]
    );
    assert_eq!(Ibm1130CallConv::callee_saved(), &[Reg::Xr2]);
}

#[test]
fn calling_convention_no_separate_frame_pointer() {
    // XR2 is the frame base AND the logical SP; there is no
    // separate FP. See docs/abi.md Sec 6.
    assert_eq!(Ibm1130CallConv::frame_pointer(), None);
    assert_eq!(Ibm1130CallConv::stack_pointer(), Reg::Xr2);
    assert_eq!(Ibm1130CallConv::STACK_ALIGNMENT, 1);
}

#[test]
fn xr3_is_never_named_in_calling_convention() {
    // XR3 is the LIBF transfer-vector base, reserved for the program
    // lifetime by the loader. It must never appear in arg, return,
    // caller-saved, callee-saved, or pointer slots.
    assert!(!Ibm1130CallConv::arg_regs().contains(&Reg::Xr3));
    assert_ne!(Ibm1130CallConv::return_reg(), Reg::Xr3);
    assert!(!Ibm1130CallConv::caller_saved().contains(&Reg::Xr3));
    assert!(!Ibm1130CallConv::callee_saved().contains(&Reg::Xr3));
    assert_ne!(Ibm1130CallConv::stack_pointer(), Reg::Xr3);
    assert_ne!(Ibm1130CallConv::frame_pointer(), Some(Reg::Xr3));
}

#[test]
fn caller_and_callee_saved_partition_xr1_xr2_acc_ext() {
    // ACC, EXT, XR1, XR2 must each appear in exactly one of
    // caller_saved or callee_saved. XR3 and IAR are intentionally
    // absent from both (XR3 is reserved-for-loader; IAR is hardware-
    // managed).
    let caller: Vec<Reg> = Ibm1130CallConv::caller_saved().to_vec();
    let callee: Vec<Reg> = Ibm1130CallConv::callee_saved().to_vec();
    for r in [Reg::Acc, Reg::Ext, Reg::Xr1, Reg::Xr2] {
        let in_caller = caller.contains(&r);
        let in_callee = callee.contains(&r);
        assert!(
            in_caller ^ in_callee,
            "{:?} must be in exactly one of caller/callee saved",
            r
        );
    }
    assert!(!caller.contains(&Reg::Xr3));
    assert!(!callee.contains(&Reg::Xr3));
    assert!(!caller.contains(&Reg::Iar));
    assert!(!callee.contains(&Reg::Iar));
}

#[test]
fn register_classes_gpr_is_acc_xr1() {
    assert_eq!(Ibm1130RegClasses::gpr(), &[Reg::Acc, Reg::Xr1]);
}

#[test]
fn register_classes_reserved_blocks_libf_base_frame_pc_and_ext() {
    // EXT, XR2, XR3, IAR all unsafe for individual allocation.
    let reserved = Ibm1130RegClasses::reserved();
    for r in [Reg::Ext, Reg::Xr2, Reg::Xr3, Reg::Iar] {
        assert!(reserved.contains(&r), "{:?} should be reserved", r);
    }
}

#[test]
fn xr3_is_reserved_class() {
    // Defensive: the LIBF transfer-vector base must always be
    // reserved. A future contributor "freeing up" XR3 would silently
    // break interop with every IBM library subroutine.
    assert!(Ibm1130RegClasses::reserved().contains(&Reg::Xr3));
    assert!(!Ibm1130RegClasses::gpr().contains(&Reg::Xr3));
}

#[test]
fn register_classes_fixed_pairs_acc_ext() {
    assert_eq!(Ibm1130RegClasses::fixed_pairs(), &[(Reg::Acc, Reg::Ext)]);
}

#[test]
fn gpr_and_reserved_are_disjoint() {
    let gpr = Ibm1130RegClasses::gpr();
    let reserved = Ibm1130RegClasses::reserved();
    for r in gpr {
        assert!(
            !reserved.contains(r),
            "{:?} must not be in both gpr and reserved",
            r
        );
    }
}

#[test]
fn marker_constructs() {
    let _ = Ibm1130Target;
    let _ = Ibm1130CallConv;
    let _ = Ibm1130RegClasses;
}
