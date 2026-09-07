//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `ldexp_q2`
//!
//! The translation is intentionally literal: it reproduces the original
//! control flow, the order of operations of the floating point arithmetic and
//! even the platform behaviour the C code relies on in cases the C standard
//! leaves undefined. No bugs are "fixed".

use std::ffi::{c_float, c_int};

/// Fractional scale factors for quarter-exponent steps.
///
/// Mirrors the function-local `static const float g_expfrac[4]` of the C
/// original. The values are `2^-30 * 2^(-k/4)` for `k = 0..3`, which is why the
/// companion integer factor below is built from `1 << 30`.
const G_EXPFRAC: [c_float; 4] = [
    9.313_225_75e-10,
    7.831_458_14e-10,
    6.585_445_08e-10,
    5.537_677_16e-10,
];

/// Scale `y` by `2^(exp_q2 / 4)`, where the exponent is given in quarter steps.
///
/// C original:
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
/// Notes on the faithful reproduction of the C semantics:
///
/// * The loop is a `do`/`while`, so the body always runs at least once, even
///   for `exp_q2 <= 0`.
/// * `e` is `min(exp_q2, 120)`; large exponents are therefore applied in
///   chunks of 120 quarter-steps (30 binary exponents) per iteration, which is
///   what keeps the integer factor `1 << 30` in range.
/// * `e & 3` is evaluated on a two's complement `int`, so a negative `e` still
///   yields an index in `0..=3` (e.g. `-3 & 3 == 1`). Rust's `&` on `i32`
///   behaves identically, so the (arguably surprising) index selection for
///   negative exponents is preserved.
/// * `e >> 2` is an arithmetic shift for negative `e` (gcc's documented
///   behaviour, and what Rust does for `i32`).
/// * For negative `e` the shift count `e >> 2` is negative, which is undefined
///   behaviour in C. The compiled C library performs the shift with `sar
///   edx, cl`, and x86 masks the count of a 32-bit shift to its low 5 bits.
///   That masking is reproduced explicitly with `& 31` so the results match
///   the C shared object bit for bit instead of panicking or differing.
/// * The multiplication is ordered exactly as written in C: the `f32` table
///   entry is multiplied by the widened integer factor first, and only that
///   product is multiplied into `y`. Since `f32` multiplication is not
///   associative, this ordering matters for the returned bit pattern.
/// * `exp_q2 -= e` can never overflow (`e` is `min(exp_q2, 120)`), but
///   `wrapping_sub` is used so the arithmetic can never panic in debug builds
///   either.
#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(y: c_float, exp_q2: c_int) -> c_float {
    let mut y = y;
    let mut exp_q2 = exp_q2;

    loop {
        // e = ((30 * 4) > exp_q2 ? exp_q2 : (30 * 4))  --  i.e. min(exp_q2, 120)
        let e: c_int = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };

        // 1 << 30 >> (e >> 2)   parses as   ((1 << 30) >> (e >> 2))
        // with the shift count truncated to 5 bits by the hardware.
        let shifted: c_int = (1i32 << 30) >> ((e >> 2) & 31);

        // y *= g_expfrac[e & 3] * (float)shifted;
        y *= G_EXPFRAC[(e & 3) as usize] * (shifted as c_float);

        exp_q2 = exp_q2.wrapping_sub(e);
        if exp_q2 <= 0 {
            break;
        }
    }

    y
}
