//! Translation of c_src/src/lib.c
//!
//! The original C code is a shared library exposing a single function
//! `ldexp_q2`. There is no `main` in the C source, so the executable
//! built from this crate produces no output (matching a C program with
//! the same library compiled and never invoked).

/// Multiply `y` by `2^(exp_q2 / 4)` using the same iterative algorithm as
/// the C implementation.  Reproduces the exact arithmetic (including the
/// quirky truncation behavior) so results are byte-identical to the C
/// version for the same `f32` inputs.
pub fn ldexp_q2(mut y: f32, mut exp_q2: i32) -> f32 {
    // Mirrors the static const float g_expfrac[4] in C.
    const G_EXPFRAC: [f32; 4] = [
        9.31322575e-10_f32,
        7.83145814e-10_f32,
        6.58544508e-10_f32,
        5.53767716e-10_f32,
    ];

    loop {
        // e = min(30 * 4, exp_q2)
        let e: i32 = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };

        // Compute (1 << 30 >> (e >> 2)) using i32 to mirror the C int
        // semantics, then convert to f32 for the multiplication.
        let shifted: i32 = (1_i32 << 30) >> (e >> 2);
        let factor: f32 = G_EXPFRAC[(e & 3) as usize] * (shifted as f32);
        y *= factor;

        // do { ... } while ((exp_q2 -= e) > 0);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
