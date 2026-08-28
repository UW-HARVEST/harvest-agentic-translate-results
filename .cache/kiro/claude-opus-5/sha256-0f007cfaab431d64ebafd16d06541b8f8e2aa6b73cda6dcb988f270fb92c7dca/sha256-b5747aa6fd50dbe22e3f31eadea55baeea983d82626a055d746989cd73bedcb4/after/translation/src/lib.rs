//! Rust translation of `c_src/src/lib.c` (xorshift128+ style PRNG producing
//! doubles in `[0.0, 1.0)`).
//!
//! Behaviour is bit-for-bit identical to the C original, including the
//! type-punning trick used to build the double from raw bits.

use std::ffi::c_double;

/// Mirrors the C `cn_rnd_t` struct:
///
/// ```c
/// typedef struct cn_rnd_t {
///     uint64_t state[2];
/// } cn_rnd_t;
/// ```
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

/// `static uint64_t cn_rnd_next(cn_rnd_t *rnd)`
///
/// Not exported (it is `static` in C). All arithmetic in C on `uint64_t` is
/// modular, hence the explicit wrapping operations here.
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

/// `double next_double(cn_rnd_t *rnd)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double {
    // The C code dereferences unconditionally; do the same.
    let rnd = unsafe { &mut *rnd };

    let value: u64 = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;

    // C: `*(double *)&result - 1.0`
    f64::from_bits(result) - 1.0
}
