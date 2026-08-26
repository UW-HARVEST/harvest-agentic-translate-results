//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `c_src/include/lib.h`):
//!   `float ldexp_q2(float y, int exp_q2);`
//!
//! The translation reproduces the original C semantics exactly, including the
//! quirky/undefined-behaviour corners that the reference C build (gcc on
//! x86-64) exhibits. See `ldexp_q2` for details.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;

/// `static const float g_expfrac[4]` from `c_src/src/lib.c`.
///
/// Bit patterns match the C build's `.rodata` exactly:
///   0x30800000, 0x305744fd, 0x303504f3, 0x301837f0
static G_EXPFRAC: [f32; 4] = [
    9.31322575e-10f32,
    7.83145814e-10f32,
    6.58544508e-10f32,
    5.53767716e-10f32,
];

/// Translation of:
///
/// ```c
/// float ldexp_q2(float y, int exp_q2) {
///     static const float g_expfrac[4] = { ... };
///     int e;
///     do {
///         e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4));
///         y *= g_expfrac[e & 3] * (1 << 30 >> (e >> 2));
///     } while ((exp_q2 -= e) > 0);
///     return y;
/// }
/// ```
///
/// Faithfulness notes:
///
/// * The body is a `do { } while` loop: it always executes at least once, even
///   for `exp_q2 <= 0`.
/// * `e = min(120, exp_q2)` — `30 * 4` folds to `120`.
/// * `e & 3` is a mask with a positive constant, so it always yields `0..=3`
///   even for negative `e` (two's complement); the array index can never go
///   out of bounds.
/// * `e >> 2` is an arithmetic right shift (gcc's implementation-defined
///   behaviour for negative values), so it sign-extends.
/// * `1 << 30 >> (e >> 2)` is a *negative* shift count whenever `e < 0`, which
///   is undefined behaviour in C. The reference build emits `sar %cl, %edx`,
///   and x86 masks the shift count to its low 5 bits. We reproduce that
///   exactly by masking with `& 31` rather than "fixing" the UB.
/// * The `int` result of the shift is converted to `float` (`cvtsi2ss`) and
///   multiplied by `g_expfrac[e & 3]`; that single-precision product is then
///   multiplied into `y`. Both multiplies happen at `f32` precision (SSE,
///   `FLT_EVAL_METHOD == 0`), matching the C build's `mulss`/`mulss` pair, so
///   rounding is bit-identical.
/// * `exp_q2 -= e` can never overflow (`e` is either `120` with
///   `exp_q2 > 120`, or `exp_q2` itself), but `wrapping_sub` is used so that
///   behaviour is identical in debug and release builds.
#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(y: f32, exp_q2: c_int) -> f32 {
    let mut y: f32 = y;
    let mut exp_q2: i32 = exp_q2;

    loop {
        // e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4));
        let e: i32 = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };

        // 1 << 30 >> (e >> 2), with the x86 5-bit shift-count masking that the
        // reference build relies on for negative counts.
        let shifted: i32 = (1i32 << 30) >> ((e >> 2) as u32 & 31);

        // y *= g_expfrac[e & 3] * (float)shifted;
        y *= G_EXPFRAC[(e & 3) as usize] * (shifted as f32);

        // while ((exp_q2 -= e) > 0);
        exp_q2 = exp_q2.wrapping_sub(e);
        if exp_q2 <= 0 {
            break;
        }
    }

    y
}
