//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing a
//! xorshift128+ style pseudo-random number generator through the public header
//! `include/lib.h`. The complete exported ABI is the single function
//! `next_double`.

#![allow(non_camel_case_types)]

use core::ptr;

/// Mirrors:
/// ```c
/// typedef struct cn_rnd_t {
///     uint64_t state[2];
/// } cn_rnd_t;
/// ```
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

/// Translation of the `static` helper:
///
/// ```c
/// static uint64_t cn_rnd_next(cn_rnd_t *rnd) {
///     uint64_t x = rnd->state[0];
///     uint64_t y = rnd->state[1];
///     rnd->state[0] = y;
///     x ^= x << 23;
///     x ^= x >> 17;
///     x ^= y ^ (y >> 26);
///     rnd->state[1] = x;
///     return x + y;
/// }
/// ```
///
/// Kept private, exactly like the C `static` function (it is not part of the
/// exported ABI).
///
/// Two deliberate details keep this bit-identical to the compiled C on every
/// input a C caller can pass:
///
/// * All arithmetic uses wrapping semantics, matching C's unsigned integer
///   overflow rules (`x + y` is `mod 2^64`). This must hold even when the
///   crate is built with `overflow-checks = on`.
/// * The two `uint64_t` slots are accessed with `read_unaligned` /
///   `write_unaligned` instead of through a `&mut cn_rnd_t` reference. `gcc`
///   and `clang` compile `rnd->state[i]` on x86-64 to alignment-agnostic 64-bit
///   `mov`s, so C happily accepts a misaligned `cn_rnd_t *`; forming a Rust
///   reference would instead trip rustc's `debug-assertions` alignment (and
///   null) checks and abort, which is observably different behaviour. Using raw
///   unaligned accesses reproduces the C exactly: a misaligned pointer works,
///   and a null pointer faults with `SIGSEGV` just like the C does.
unsafe fn cn_rnd_next(rnd: *mut cn_rnd_t) -> u64 {
    // `cn_rnd_t` is `#[repr(C)]` wrapping `[u64; 2]`, so the struct pointer and
    // the pointer to `state[0]` have the same address.
    let state = rnd as *mut u64;

    let mut x: u64 = unsafe { ptr::read_unaligned(state) };
    let y: u64 = unsafe { ptr::read_unaligned(state.add(1)) };
    unsafe { ptr::write_unaligned(state, y) };
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    unsafe { ptr::write_unaligned(state.add(1), x) };
    x.wrapping_add(y)
}

/// Translation of:
///
/// ```c
/// double next_double(cn_rnd_t *rnd) {
///     uint64_t value = cn_rnd_next(rnd);
///     uint64_t exponent = 1023;
///     uint64_t mantissa = value >> 12;
///     uint64_t result = (exponent << 52) | mantissa;
///     return *(double *)&result - 1.0;
/// }
/// ```
///
/// The type-punning `*(double *)&result` is reproduced bit-for-bit with
/// `f64::from_bits`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> f64 {
    let value: u64 = unsafe { cn_rnd_next(rnd) };
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
