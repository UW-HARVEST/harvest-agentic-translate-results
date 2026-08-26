//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   - `next_double`
//!
//! `cn_rnd_next` is `static` in the C source, so it is not exported; it is
//! reproduced here as a private helper with identical semantics.

use std::ffi::c_double;

/// Mirror of the C `cn_rnd_t`:
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

/// Translation of the `static` C helper:
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
/// The shifts and the final addition use wrapping arithmetic to match C's
/// unsigned 64-bit semantics exactly.
#[inline]
fn cn_rnd_next(rnd: &mut cn_rnd_t) -> u64 {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

/// `double next_double(cn_rnd_t *rnd);`
///
/// Reproduces the C body verbatim, including the type-punning read of the
/// assembled bit pattern as an IEEE-754 double:
///
/// ```c
/// uint64_t value = cn_rnd_next(rnd);
/// uint64_t exponent = 1023;
/// uint64_t mantissa = value >> 12;
/// uint64_t result = (exponent << 52) | mantissa;
/// return *(double *)&result - 1.0;
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double {
    // The C code dereferences `rnd` unconditionally with no NULL check; we
    // preserve that behaviour rather than "fixing" it.
    let rnd = unsafe { &mut *rnd };

    let value: u64 = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
