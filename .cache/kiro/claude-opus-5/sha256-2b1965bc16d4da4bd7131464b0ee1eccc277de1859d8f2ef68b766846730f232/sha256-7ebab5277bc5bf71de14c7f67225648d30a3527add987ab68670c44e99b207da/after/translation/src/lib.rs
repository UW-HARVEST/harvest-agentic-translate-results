//! Rust translation of `c_src/src/lib.c` (tflac bitwriter fragment).
//!
//! The translation is intentionally literal: every arithmetic operation uses
//! the same width and the same wrapping behaviour as the original C, and the
//! order of operations is preserved exactly. Bugs present in the C source are
//! reproduced rather than fixed.

use std::ffi::c_int;

// C typedefs from include/lib.h
#[allow(non_camel_case_types)]
pub type tflac_u8 = u8;
#[allow(non_camel_case_types)]
pub type tflac_u32 = u32;
#[allow(non_camel_case_types)]
pub type tflac_u64 = u64;
#[allow(non_camel_case_types)]
pub type tflac_uint = tflac_u64;

/// Mirrors `struct tflac_bitwriter` from include/lib.h.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct tflac_bitwriter {
    pub val: tflac_uint,
    pub bits: tflac_u32,
    pub pos: tflac_u32,
    pub len: tflac_u32,
    pub tot: tflac_u32,
    pub buffer: *mut tflac_u8,
}

/// `8 * sizeof(tflac_uint)` as evaluated in C: a `size_t` (u64) value of 64.
const UINT_BITS: tflac_u64 = 8 * (core::mem::size_of::<tflac_uint>() as tflac_u64);

/// Reproduces the C shift semantics on x86-64: the shift count is taken
/// modulo the operand width, which is what the hardware does for the
/// technically-undefined out-of-range shifts the C code can perform.
#[inline]
fn shl_u64(v: tflac_u64, n: tflac_u64) -> tflac_u64 {
    v.wrapping_shl(n as u32)
}

#[inline]
fn shr_u64(v: tflac_u64, n: tflac_u64) -> tflac_u64 {
    v.wrapping_shr(n as u32)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut tflac_bitwriter,
    bits: tflac_u32,
    val: tflac_uint,
) -> c_int {
    // const tflac_uint mask = (18446744073709551615UL) << 1;
    let mask: tflac_uint = 18446744073709551615u64 << 1;

    let bw = unsafe { &mut *bw };

    // Local mutable copies of the by-value C parameters.
    let mut bits: tflac_u32 = bits;
    let mut val: tflac_uint = val;
    let mut b: tflac_u32;

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    // Computed in u64 (size_t) arithmetic, wrapping if bits > 64.
    val = shl_u64(val, UINT_BITS.wrapping_sub(bits as tflac_u64));

    // bw->tot += bits;
    bw.tot = bw.tot.wrapping_add(bits);

    let mut i: c_int = 0;

    // while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100)
    // `bw->bits + bits` is unsigned int (u32) arithmetic and wraps, then it is
    // widened to size_t for the comparison against 64.
    while (bw.bits.wrapping_add(bits) as tflac_u64) >= UINT_BITS && i < 100 {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;  (u64 math, stored in u32)
        b = UINT_BITS
            .wrapping_sub(bw.bits as tflac_u64)
            .wrapping_sub(1) as tflac_u32;

        // b = b > bits ? bits : b;
        b = if b > bits { bits } else { b };

        // bw->val |= (val >> bw->bits);
        bw.val |= shr_u64(val, bw.bits as tflac_u64);

        // bw->bits += b;
        bw.bits = bw.bits.wrapping_add(b);

        // bw->val &= mask;
        bw.val &= mask;

        // val <<= b;
        val = shl_u64(val, b as tflac_u64);

        // bits -= b;
        bits = bits.wrapping_sub(b);

        i = i.wrapping_add(1);
    }

    // bw->val |= (val >> bw->bits);
    bw.val |= shr_u64(val, bw.bits as tflac_u64);

    // bw->bits += bits;
    bw.bits = bw.bits.wrapping_add(bits);

    0
}
