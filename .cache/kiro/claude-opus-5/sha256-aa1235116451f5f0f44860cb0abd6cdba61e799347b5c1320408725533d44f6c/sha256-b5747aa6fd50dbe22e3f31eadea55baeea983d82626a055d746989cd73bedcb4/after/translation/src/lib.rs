//! Rust translation of `c_src/src/lib.c`.
//!
//! Mirrors the original C semantics exactly, including the do-while structure,
//! the operand order of the floating point multiplications, and the behaviour of
//! the integer shift for negative exponents.

use std::ffi::c_int;

/// Same table as the `static const float g_expfrac[4]` in the C translation unit.
const G_EXPFRAC: [f32; 4] = [
    9.313_225_75e-10,
    7.831_458_14e-10,
    6.585_445_08e-10,
    5.537_677_16e-10,
];

/// ```c
/// float ldexp_q2(float y, int exp_q2);
/// ```
///
/// The header declares no namespace/renaming macro, so the linker symbol is
/// plain `ldexp_q2`.
#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(y: f32, exp_q2: c_int) -> f32 {
    let mut y = y;
    let mut exp_q2 = exp_q2;

    loop {
        // e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4))  ->  min(exp_q2, 120)
        let e: c_int = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };

        // (1 << 30) >> (e >> 2) as plain int arithmetic. For a negative `e` the
        // shift count is negative, which is UB in C; on the platforms the C code
        // targets the hardware shift masks the count to its low 5 bits, so
        // `wrapping_shr` (which masks with & 31) reproduces that behaviour.
        let scale: c_int = (1i32 << 30).wrapping_shr((e >> 2) as u32);

        // `e & 3` on a negative `e` keeps two's-complement semantics in both
        // languages, so the index always lands in 0..=3.
        let idx = (e & 3) as usize;

        // Keep the C evaluation order: table entry times the integer scale
        // (converted to float) first, then the multiply-assign into y.
        y *= G_EXPFRAC[idx] * (scale as f32);

        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }

    y
}
