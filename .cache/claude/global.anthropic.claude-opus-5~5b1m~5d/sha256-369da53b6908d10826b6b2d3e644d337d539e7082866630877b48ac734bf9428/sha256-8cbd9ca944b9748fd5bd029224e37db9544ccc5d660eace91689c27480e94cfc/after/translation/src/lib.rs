//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface of the C shared library (as reported by `nm -D`):
//!   * `bitwriter_add`
//!
//! The C sources are translated verbatim, including behaviour that is
//! technically undefined in C (out-of-range shift counts, unsigned
//! wrap-around).  Those constructs are reproduced exactly as the reference
//! compiler emits them on x86-64:
//!   * shift counts are taken modulo the operand width (`shl`/`shr` with `%cl`),
//!     which is what `wrapping_shl` / `wrapping_shr` do in Rust;
//!   * `unsigned int` arithmetic wraps modulo 2^32 (`wrapping_*`).
//! No bugs are fixed and the order of every operation is preserved.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;
pub type tflac_uint = tflac_u64;

/// struct tflac_bitwriter
///
/// Layout (x86-64):
///   val    @ 0x00 (8 bytes)
///   bits   @ 0x08 (4 bytes)
///   pos    @ 0x0c (4 bytes)
///   len    @ 0x10 (4 bytes)
///   tot    @ 0x14 (4 bytes)
///   buffer @ 0x18 (8 bytes)   => sizeof == 32, alignof == 8
#[repr(C)]
pub struct tflac_bitwriter {
    pub val: tflac_uint,
    pub bits: tflac_u32,
    pub pos: tflac_u32,
    pub len: tflac_u32,
    pub tot: tflac_u32,
    pub buffer: *mut tflac_u8,
}

/// Number of bits in a `tflac_uint`, i.e. `8 * sizeof(tflac_uint)`.
const UINT_BITS: tflac_u32 = 8 * std::mem::size_of::<tflac_uint>() as tflac_u32; // 64

// ---------------------------------------------------------------------------
// src/lib.c
// ---------------------------------------------------------------------------

/// ```c
/// int bitwriter_add(tflac_bitwriter *bw, tflac_u32 bits, tflac_uint val);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut tflac_bitwriter,
    bits: tflac_u32,
    val: tflac_uint,
) -> c_int {
    // const tflac_uint mask = (18446744073709551615UL) << 1;
    const MASK: tflac_uint = 0xFFFF_FFFF_FFFF_FFFFu64 << 1;

    let mut bits: tflac_u32 = bits;
    let mut val: tflac_uint = val;
    let b: &mut tflac_bitwriter = unsafe { &mut *bw };

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    // The shift count is reduced modulo 64 by the hardware shift instruction.
    val = val.wrapping_shl(UINT_BITS.wrapping_sub(bits));

    // bw->tot += bits;
    b.tot = b.tot.wrapping_add(bits);

    // int i = 0;
    let mut i: c_int = 0;

    // while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
    while b.bits.wrapping_add(bits) >= UINT_BITS && i < 100 {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;
        let mut bb: tflac_u32 = UINT_BITS.wrapping_sub(b.bits).wrapping_sub(1);

        // b = b > bits ? bits : b;
        bb = if bb > bits { bits } else { bb };

        // bw->val |= (val >> bw->bits);
        b.val |= val.wrapping_shr(b.bits);

        // bw->bits += b;
        b.bits = b.bits.wrapping_add(bb);

        // bw->val &= mask;
        b.val &= MASK;

        // val <<= b;
        val = val.wrapping_shl(bb);

        // bits -= b;
        bits = bits.wrapping_sub(bb);

        // i++;
        i = i.wrapping_add(1);
    }

    // bw->val |= (val >> bw->bits);
    b.val |= val.wrapping_shr(b.bits);

    // bw->bits += bits;
    b.bits = b.bits.wrapping_add(bits);

    // return 0;
    0
}
