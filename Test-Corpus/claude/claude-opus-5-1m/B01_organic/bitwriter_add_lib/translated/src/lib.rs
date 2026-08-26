//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`c_src/src/lib.c`) with
//! the public header `c_src/include/lib.h`. It exports exactly one public
//! symbol: `bitwriter_add`.
//!
//! The translation reproduces the observable behaviour of the compiled C code
//! bit-for-bit, including its buggy / implementation-defined corners:
//!
//! * All arithmetic on the `tflac_u32` / `tflac_uint` values wraps, exactly as
//!   unsigned C arithmetic does.
//! * Shift counts are masked to the low 6 bits, which is what the x86-64
//!   `shlq`/`shrq` instructions emitted by the C compiler do when the shift
//!   count is out of range (e.g. `val <<= 64 - bits` with `bits == 0`).
//! * The `i < 100` loop bound, the `bw->val &= mask` clearing of the low bit,
//!   and the off-by-one `b = 64 - bw->bits - 1` are all preserved verbatim.

#![allow(non_camel_case_types)]

use core::ffi::c_int;

/// `struct tflac_bitwriter` from `c_src/include/lib.h`.
///
/// Layout (x86-64): `val` @ 0, `bits` @ 8, `pos` @ 12, `len` @ 16, `tot` @ 20,
/// `buffer` @ 24; size 32, align 8.
#[repr(C)]
pub struct tflac_bitwriter {
    /// `tflac_uint val;`
    pub val: u64,
    /// `tflac_u32 bits;`
    pub bits: u32,
    /// `tflac_u32 pos;`
    pub pos: u32,
    /// `tflac_u32 len;`
    pub len: u32,
    /// `tflac_u32 tot;`
    pub tot: u32,
    /// `tflac_u8 *buffer;`
    pub buffer: *mut u8,
}

/// Number of bits in a `tflac_uint` (`8 * sizeof(tflac_uint)`).
const TFLAC_UINT_BITS: u32 = 8 * core::mem::size_of::<u64>() as u32;

/// `val << n` with the hardware (x86-64) shift-count masking of the C compiler.
#[inline(always)]
fn shl(val: u64, n: u32) -> u64 {
    val << (n & (TFLAC_UINT_BITS - 1))
}

/// `val >> n` with the hardware (x86-64) shift-count masking of the C compiler.
#[inline(always)]
fn shr(val: u64, n: u32) -> u64 {
    val >> (n & (TFLAC_UINT_BITS - 1))
}

/// ```c
/// int bitwriter_add(tflac_bitwriter *bw, tflac_u32 bits, tflac_uint val);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut tflac_bitwriter,
    bits: u32,
    val: u64,
) -> c_int {
    // const tflac_uint mask = (18446744073709551615UL) << 1;
    const MASK: u64 = u64::MAX << 1;

    let bw: &mut tflac_bitwriter = unsafe { &mut *bw };

    let mut bits: u32 = bits;
    let mut val: u64 = val;

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    val = shl(val, TFLAC_UINT_BITS.wrapping_sub(bits));

    // bw->tot += bits;
    bw.tot = bw.tot.wrapping_add(bits);

    // int i = 0;
    let mut i: c_int = 0;

    // while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
    while bw.bits.wrapping_add(bits) >= TFLAC_UINT_BITS && i < 100 {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;
        let mut b: u32 = TFLAC_UINT_BITS.wrapping_sub(bw.bits).wrapping_sub(1);

        // b = b > bits ? bits : b;
        b = if b > bits { bits } else { b };

        // bw->val |= (val >> bw->bits);
        bw.val |= shr(val, bw.bits);

        // bw->bits += b;
        bw.bits = bw.bits.wrapping_add(b);

        // bw->val &= mask;
        bw.val &= MASK;

        // val <<= b;
        val = shl(val, b);

        // bits -= b;
        bits = bits.wrapping_sub(b);

        // i++;
        i += 1;
    }

    // bw->val |= (val >> bw->bits);
    bw.val |= shr(val, bw.bits);

    // bw->bits += bits;
    bw.bits = bw.bits.wrapping_add(bits);

    // return 0;
    0
}
