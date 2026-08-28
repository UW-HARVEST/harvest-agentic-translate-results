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

    // NOTE ON HOW `bw` IS DEREFERENCED
    //
    // The C performs `bw->tot += bits` with no null check, so `bw == NULL` is a
    // plain unchecked dereference that faults with SIGSEGV.  Reproducing that
    // fault mode in *every* Cargo profile rules out two obvious spellings:
    //
    //   * `&mut *bw` — forming a Rust reference makes rustc's reference-validity
    //     debug assertion fire, aborting with SIGABRT instead of faulting.
    //   * `(*bw).field` — a raw place deref still gets rustc's MIR `CheckNull`
    //     instrumentation under `-C debug-assertions` (i.e. `cargo build`), which
    //     also turns the fault into a SIGABRT panic.
    //
    // Both would be an observable difference from the C across the FFI boundary
    // (C = SIGSEGV/11, Rust = SIGABRT/6).  Instead we take raw field addresses
    // with `&raw mut` (address arithmetic only, never a dereference, so it is not
    // instrumented) and perform the accesses with `ptr::read` / `ptr::write`,
    // whose dereference lives in precompiled `core` and so is not instrumented
    // either.  These lower to bare loads/stores, exactly like `bw->field` in C.
    //
    // Field accesses below are ordered exactly as in the C source.
    let p_val: *mut tflac_uint = unsafe { &raw mut (*bw).val };
    let p_bits: *mut tflac_u32 = unsafe { &raw mut (*bw).bits };
    let p_tot: *mut tflac_u32 = unsafe { &raw mut (*bw).tot };

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    // The shift count is reduced modulo 64 by the hardware shift instruction.
    val = val.wrapping_shl(UINT_BITS.wrapping_sub(bits));

    // bw->tot += bits;
    unsafe { p_tot.write(p_tot.read().wrapping_add(bits)) };

    // int i = 0;
    let mut i: c_int = 0;

    // while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
    while unsafe { p_bits.read() }.wrapping_add(bits) >= UINT_BITS && i < 100 {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;
        let mut bb: tflac_u32 =
            UINT_BITS.wrapping_sub(unsafe { p_bits.read() }).wrapping_sub(1);

        // b = b > bits ? bits : b;
        bb = if bb > bits { bits } else { bb };

        // bw->val |= (val >> bw->bits);
        unsafe { p_val.write(p_val.read() | val.wrapping_shr(p_bits.read())) };

        // bw->bits += b;
        unsafe { p_bits.write(p_bits.read().wrapping_add(bb)) };

        // bw->val &= mask;
        unsafe { p_val.write(p_val.read() & MASK) };

        // val <<= b;
        val = val.wrapping_shl(bb);

        // bits -= b;
        bits = bits.wrapping_sub(bb);

        // i++;
        i = i.wrapping_add(1);
    }

    // bw->val |= (val >> bw->bits);
    unsafe { p_val.write(p_val.read() | val.wrapping_shr(p_bits.read())) };

    // bw->bits += bits;
    unsafe { p_bits.write(p_bits.read().wrapping_add(bits)) };

    // return 0;
    0
}
