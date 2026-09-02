//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   * `bitwriter_add`
//!
//! The header `include/lib.h` contains no namespace-renaming preprocessor
//! macros, so the linker symbol name matches the source-level name exactly.

#![allow(non_camel_case_types)]

use core::ffi::c_int;

/* ------------------------------------------------------------------ */
/* Typedefs from include/lib.h                                        */
/* ------------------------------------------------------------------ */

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;
/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;
/// `typedef uint64_t tflac_u64;`
pub type tflac_u64 = u64;
/// `typedef tflac_u64 tflac_uint;`
pub type tflac_uint = tflac_u64;

/// `8 * sizeof(tflac_uint)` as evaluated by the C compiler.
///
/// In C this expression has type `size_t` (unsigned long), which matters for
/// the integer promotions in the translated arithmetic below.
const TFLAC_UINT_BITS: u64 = 8 * core::mem::size_of::<tflac_uint>() as u64; // 64

/* ------------------------------------------------------------------ */
/* struct tflac_bitwriter                                             */
/*                                                                    */
/* Verified against the C ABI on this target:                         */
/*   size=32 val=0 bits=8 pos=12 len=16 tot=20 buffer=24              */
/* ------------------------------------------------------------------ */

#[repr(C)]
pub struct tflac_bitwriter {
    pub val: tflac_uint,
    pub bits: tflac_u32,
    pub pos: tflac_u32,
    pub len: tflac_u32,
    pub tot: tflac_u32,
    pub buffer: *mut tflac_u8,
}

/* ------------------------------------------------------------------ */
/* Shift helpers                                                      */
/*                                                                    */
/* The C code performs shifts of a 64-bit value by counts that can be */
/* >= 64 (e.g. `val <<= 64 - bits` when `bits == 0`). That is UB in C, */
/* but the generated code uses the x86-64 `shl %cl` / `shr %cl`       */
/* instructions, which mask the shift count to its low 6 bits. These  */
/* helpers reproduce that observed behaviour bit-for-bit so the Rust   */
/* port yields byte-identical results.                                */
/* ------------------------------------------------------------------ */

#[inline(always)]
fn c_shl_u64(v: tflac_uint, count: u64) -> tflac_uint {
    v << (count & 63)
}

#[inline(always)]
fn c_shr_u64(v: tflac_uint, count: u64) -> tflac_uint {
    v >> (count & 63)
}

/* ------------------------------------------------------------------ */
/* src/lib.c                                                          */
/* ------------------------------------------------------------------ */

/// Translation of:
///
/// ```c
/// int bitwriter_add(tflac_bitwriter *bw, tflac_u32 bits, tflac_uint val);
/// ```
///
/// Behaviour is reproduced exactly, including the unused local `r`, the
/// `i < 100` loop guard, the unsigned wrap-around in `bits -= b`, and the
/// out-of-range shifts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut tflac_bitwriter,
    bits: tflac_u32,
    val: tflac_uint,
) -> c_int {
    // const tflac_uint mask = (18446744073709551615UL) << 1;
    const MASK: tflac_uint = 18446744073709551615u64 << 1; // 0xFFFF_FFFF_FFFF_FFFE

    // Mutable copies of the by-value parameters.
    let mut bits: tflac_u32 = bits;
    let mut val: tflac_uint = val;

    // tflac_u32 b;
    let mut b: tflac_u32;
    // int r;  -- declared but never used in the C source.
    let _r: c_int;

    // The C code accesses `bw->...` directly through the caller-supplied
    // pointer. We deliberately do NOT form a `&mut tflac_bitwriter` here:
    // creating a Rust reference from a raw pointer asserts non-null and
    // aligned, and rustc's debug-mode UB checks turn a NULL `bw` into a panic
    // (which, escaping an `extern "C"` fn, aborts with SIGABRT). The C has no
    // null check and simply faults with SIGSEGV. Using raw place expressions
    // reproduces the C behaviour identically in every profile, and also avoids
    // claiming the `noalias` guarantee that the C pointer does not carry.

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    //   `8 * sizeof(...)` is size_t, so the subtraction is done in u64.
    val = c_shl_u64(val, TFLAC_UINT_BITS.wrapping_sub(bits as u64));

    // bw->tot += bits;
    unsafe { (*bw).tot = (*bw).tot.wrapping_add(bits) };

    // int i = 0;
    let mut i: c_int = 0;

    // while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
    //   `bw->bits + bits` is computed in tflac_u32 (wrapping), then widened
    //   to size_t for the comparison against 64.
    while (unsafe { (*bw).bits }.wrapping_add(bits) as u64) >= TFLAC_UINT_BITS && i < 100 {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;
        //   Computed in u64, then truncated on assignment to tflac_u32.
        b = TFLAC_UINT_BITS
            .wrapping_sub(unsafe { (*bw).bits } as u64)
            .wrapping_sub(1) as tflac_u32;

        // b = b > bits ? bits : b;
        b = if b > bits { bits } else { b };

        // bw->val |= (val >> bw->bits);
        unsafe { (*bw).val |= c_shr_u64(val, (*bw).bits as u64) };

        // bw->bits += b;
        unsafe { (*bw).bits = (*bw).bits.wrapping_add(b) };

        // bw->val &= mask;
        unsafe { (*bw).val &= MASK };

        // val <<= b;
        val = c_shl_u64(val, b as u64);

        // bits -= b;
        bits = bits.wrapping_sub(b);

        // i++;
        i = i.wrapping_add(1);
    }

    // bw->val |= (val >> bw->bits);
    unsafe { (*bw).val |= c_shr_u64(val, (*bw).bits as u64) };

    // bw->bits += bits;
    unsafe { (*bw).bits = (*bw).bits.wrapping_add(bits) };

    // return 0;
    0
}
