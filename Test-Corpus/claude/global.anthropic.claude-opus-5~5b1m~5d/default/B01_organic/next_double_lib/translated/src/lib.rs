//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing a
//! xorshift128+ style pseudo-random number generator through the public header
//! `include/lib.h`. The complete exported ABI is the single function
//! `next_double`.

#![allow(non_camel_case_types)]

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
/// exported ABI). All arithmetic uses wrapping semantics to match C's unsigned
/// integer overflow behaviour.
fn cn_rnd_next(rnd: &mut cn_rnd_t) -> u64 {
    let mut x: u64 = rnd.state[0];
    let y: u64 = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
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
    let rnd = unsafe { &mut *rnd };
    let value: u64 = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
