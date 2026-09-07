//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   - `rev16`
//!
//! Source of truth: `c_src/include/lib.h` and `c_src/src/lib.c`.
//! No namespace/renaming macros are present in the public header, so the
//! linker symbol names match the source-level names exactly.

#![allow(clippy::missing_safety_doc)]

/// Reverse the bit order of the low 16 bits of `a`.
///
/// Direct translation of:
///
/// ```c
/// uint32_t rev16(uint32_t a) {
///     a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
///     a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
///     a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
///     a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
///     return a;
/// }
/// ```
///
/// Note that the masks are all 16-bit wide, so the upper 16 bits of the
/// argument are discarded by the first statement. This behaviour is part of
/// the original C semantics and is reproduced verbatim rather than "fixed".
///
/// All intermediate values stay within 16 bits, so the shifts cannot overflow
/// a `u32`; nevertheless `wrapping_*` operations are used so that the
/// arithmetic matches C's unsigned (modular) semantics even in a debug build.
#[unsafe(no_mangle)]
pub extern "C" fn rev16(a: core::ffi::c_uint) -> core::ffi::c_uint {
    let mut a: u32 = a as u32;

    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555).wrapping_shl(1));
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333).wrapping_shl(2));
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F).wrapping_shl(4));
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF).wrapping_shl(8));

    a as core::ffi::c_uint
}

#[cfg(test)]
mod tests {
    use super::rev16;

    /// Independent reference implementation: reverse the low 16 bits and zero
    /// the upper half, mirroring what the C code computes.
    fn reference(a: u32) -> u32 {
        let low = (a & 0xFFFF) as u16;
        low.reverse_bits() as u32
    }

    #[test]
    fn matches_reference_on_known_values() {
        assert_eq!(rev16(0x0000), 0x0000);
        assert_eq!(rev16(0xFFFF), 0xFFFF);
        assert_eq!(rev16(0x0001), 0x8000);
        assert_eq!(rev16(0x8000), 0x0001);
        assert_eq!(rev16(0x1234), 0x2C48);
        // Upper 16 bits are discarded, exactly as in C.
        assert_eq!(rev16(0xDEAD_0001), 0x8000);
    }

    #[test]
    fn matches_reference_exhaustively_over_low_half() {
        for a in 0u32..=0xFFFF {
            assert_eq!(rev16(a), reference(a), "mismatch for {a:#06x}");
        }
    }

    #[test]
    fn upper_bits_are_always_ignored() {
        for a in 0u32..=0xFFFF {
            let expected = rev16(a);
            for hi in [0x0000u32, 0x0001, 0xABCD, 0xFFFF] {
                assert_eq!(rev16((hi << 16) | a), expected);
            }
        }
    }
}
