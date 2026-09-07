//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `ldexp_q2`
//!
//! The translation reproduces the original semantics exactly, including the
//! implementation-defined / undefined behaviour the C code relies on (see the
//! notes on `ldexp_q2`).

#![allow(clippy::needless_return)]

use std::ffi::c_int;

/// `static const float g_expfrac[4]` from `src/lib.c`.
///
/// These are `2^-30 * 2^(-k/4)` for `k = 0..3`.
const G_EXPFRAC: [f32; 4] = [
    9.31322575e-10f32,
    7.83145814e-10f32,
    6.58544508e-10f32,
    5.53767716e-10f32,
];

/// ```c
/// float ldexp_q2(float y, int exp_q2) {
///     static const float g_expfrac[4] = {...};
///     int e;
///     do {
///         e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4));
///         y *= g_expfrac[e & 3] * (1 << 30 >> (e >> 2));
///     } while ((exp_q2 -= e) > 0);
///     return y;
/// }
/// ```
///
/// Behavioural notes (faithfully preserved, *not* "fixed"):
///
/// * `e` is `min(120, exp_q2)`, so it may be negative. `e & 3` then indexes with
///   the two's-complement low bits (e.g. `-1 & 3 == 3`), which keeps the index
///   inside the array, and `e >> 2` is an arithmetic shift.
/// * For a negative `e`, `1 << 30 >> (e >> 2)` shifts by a negative amount,
///   which is undefined behaviour in C. The C build (gcc, x86-64) emits
///   `sar %cl`, whose count is taken modulo 32, so the observable behaviour is a
///   shift by `(e >> 2) & 31`. That is what is reproduced here; masking also
///   keeps the Rust shift itself well defined.
/// * The arithmetic is done in `f32` (x86-64 SSE, `FLT_EVAL_METHOD == 0`): the
///   fraction is first scaled by the integer power of two, then multiplied into
///   `y`.
/// * `exp_q2 -= e` can never overflow: either `e == exp_q2` (result `0`) or
///   `e == 120` with `exp_q2 > 120`.
#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(y: f32, exp_q2: c_int) -> f32 {
    let mut y = y;
    let mut exp_q2 = exp_q2;

    loop {
        // e = ((30 * 4) > exp_q2 ? exp_q2 : (30 * 4))
        let e: c_int = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };

        // g_expfrac[e & 3] * (1 << 30 >> (e >> 2))
        let frac = G_EXPFRAC[(e & 3) as usize];
        let scale = (1i32 << 30) >> ((e >> 2) & 31);
        y *= frac * (scale as f32);

        // while ((exp_q2 -= e) > 0)
        exp_q2 = exp_q2.wrapping_sub(e);
        if exp_q2 <= 0 {
            return y;
        }
    }
}
