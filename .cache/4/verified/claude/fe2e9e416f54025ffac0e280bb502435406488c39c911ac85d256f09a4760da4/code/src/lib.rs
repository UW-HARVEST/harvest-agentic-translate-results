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
//!
//! # Why the struct fields are accessed through raw unaligned loads/stores
//!
//! `bitwriter_add` takes a `tflac_bitwriter *` straight from an external
//! caller, so the pointer can be anything the C ABI permits — including an
//! **unaligned** address. The C compiler emits plain `mov` instructions for
//! `bw->val` / `bw->bits` / `bw->tot`, so on x86-64 an unaligned `bw` simply
//! works and the function still returns `0`.
//!
//! Forming a Rust reference (`&mut *bw`) would *not* match that: it requires
//! the pointer to be aligned, and a debug build turns a misaligned `bw` into
//! `misaligned pointer dereference` → `SIGABRT`, whereas the C returns `0`.
//! The same reference also makes a `NULL` `bw` abort with `SIGABRT` instead of
//! the `SIGSEGV` the C produces. Both are observable differences for an
//! external caller, so every field access below goes through
//! [`core::ptr::read_unaligned`] / [`core::ptr::write_unaligned`] on addresses
//! obtained with [`core::ptr::addr_of_mut`], which never forms a reference.
//!
//! Field-access **order** is preserved statement-for-statement as well. This
//! matters for the `NULL` case: the first memory touch in the C is
//! `bw->tot += bits` at byte offset 20, i.e. address `0x14` — a non-null
//! address — so the process dies with `SIGSEGV` exactly like the C, rather than
//! tripping a Rust null-pointer check.

#![allow(non_camel_case_types)]

use core::ffi::c_int;
use core::ptr::{addr_of_mut, read_unaligned, write_unaligned};

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

    // Addresses of the three fields the C touches. `addr_of_mut!` only does
    // pointer arithmetic; it never forms a reference, so an unaligned or null
    // `bw` is carried through to the actual load/store just like in C.
    let p_val: *mut u64 = unsafe { addr_of_mut!((*bw).val) };
    let p_bits: *mut u32 = unsafe { addr_of_mut!((*bw).bits) };
    let p_tot: *mut u32 = unsafe { addr_of_mut!((*bw).tot) };

    let mut bits: u32 = bits;
    let mut val: u64 = val;

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    val = shl(val, TFLAC_UINT_BITS.wrapping_sub(bits));

    // bw->tot += bits;
    unsafe { write_unaligned(p_tot, read_unaligned(p_tot).wrapping_add(bits)) };

    // int i = 0;
    let mut i: c_int = 0;

    // while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
    while unsafe { read_unaligned(p_bits) }.wrapping_add(bits) >= TFLAC_UINT_BITS && i < 100 {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;
        let mut b: u32 = TFLAC_UINT_BITS
            .wrapping_sub(unsafe { read_unaligned(p_bits) })
            .wrapping_sub(1);

        // b = b > bits ? bits : b;
        b = if b > bits { bits } else { b };

        // bw->val |= (val >> bw->bits);
        unsafe {
            let cur_bits = read_unaligned(p_bits);
            write_unaligned(p_val, read_unaligned(p_val) | shr(val, cur_bits));
        }

        // bw->bits += b;
        unsafe { write_unaligned(p_bits, read_unaligned(p_bits).wrapping_add(b)) };

        // bw->val &= mask;
        unsafe { write_unaligned(p_val, read_unaligned(p_val) & MASK) };

        // val <<= b;
        val = shl(val, b);

        // bits -= b;
        bits = bits.wrapping_sub(b);

        // i++;
        i += 1;
    }

    // bw->val |= (val >> bw->bits);
    unsafe {
        let cur_bits = read_unaligned(p_bits);
        write_unaligned(p_val, read_unaligned(p_val) | shr(val, cur_bits));
    }

    // bw->bits += bits;
    unsafe { write_unaligned(p_bits, read_unaligned(p_bits).wrapping_add(bits)) };

    // return 0;
    0
}
