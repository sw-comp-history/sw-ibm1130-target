//! Type widths and alignment for the IBM 1130. See `docs/abi.md` Sec 10.
//!
//! All sizes are in **address-units** (1 word = 16 bits = 2 bytes).
//! Pointer width is 16 bits. Word alignment (1 unit) is the only
//! alignment that exists on a word-addressed machine.

use sw_target_core::PrimType;

/// Width of `ty` in address-units (1130 words).
pub const fn type_width(ty: PrimType) -> usize {
    match ty {
        PrimType::I8 | PrimType::U8 | PrimType::Bool => 1,
        PrimType::I16 | PrimType::U16 => 1,
        PrimType::I32 | PrimType::U32 => 2,
        PrimType::I64 | PrimType::U64 => 4,
        PrimType::Ptr => 1,
    }
}

/// Alignment of `ty` in address-units. The 1130 is word-addressed; 1
/// is the only alignment.
pub const fn type_alignment(_ty: PrimType) -> usize {
    1
}
