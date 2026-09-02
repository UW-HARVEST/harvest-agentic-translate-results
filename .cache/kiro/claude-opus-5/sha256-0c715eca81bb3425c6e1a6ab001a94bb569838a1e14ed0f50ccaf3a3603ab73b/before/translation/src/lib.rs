//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `next_double`
//!
//! Header (`include/lib.h`):
//! ```c
//! typedef struct cn_rnd_t { uint64_t state[2]; } cn_rnd_t;
//! double next_double(cn_rnd_t *rnd);
//! ```

use std::ffi::c_double;

/// Mirrors `cn_rnd_t` from `include/lib.h`: two `uint64_t` words.
/// `#[repr(C)]` keeps layout/alignment identical to the C struct.
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

/// Translation of the file-local `static uint64_t cn_rnd_next(cn_rnd_t *rnd)`.
///
/// xorshift128+ style step. All arithmetic in C is on `uint64_t`, i.e. modulo
/// 2^64, so the final addition uses `wrapping_add` to reproduce it exactly
/// (and to avoid a debug-build overflow panic).
#[inline]
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
///
/// Builds an IEEE-754 double in [1.0, 2.0) from the top 52 bits of the
/// generated value and subtracts 1.0. The C code type-puns through
/// `*(double *)&result`; `f64::from_bits` is the exact equivalent bit
/// reinterpretation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double {
    // The C function dereferences `rnd` unconditionally (no NULL check);
    // reproduce that behaviour rather than "fixing" it.
    let rnd: &mut cn_rnd_t = unsafe { &mut *rnd };

    let value: u64 = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
