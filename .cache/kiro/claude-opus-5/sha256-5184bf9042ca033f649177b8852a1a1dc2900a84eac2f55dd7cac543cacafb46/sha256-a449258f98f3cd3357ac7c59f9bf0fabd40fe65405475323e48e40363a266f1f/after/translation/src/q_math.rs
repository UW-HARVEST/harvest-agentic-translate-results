//! Translation of the pieces of `q_math.c` that the driver exercises.

use crate::fpu::{fmul, fsub};

/// `float Q_rsqrt( float number )` — the classic fast reciprocal square root
/// with one Newton iteration. The bit-level punning is reproduced with
/// `to_bits`/`from_bits`, which is exactly what the `memcpy` calls do.
pub fn q_rsqrt(number: f32) -> f32 {
    let threehalfs: f32 = 1.5;

    let x2: f32 = fmul(number, 0.5); // x2 = number * 0.5F
    let mut y: f32 = number;

    let mut i: u32 = y.to_bits(); // evil floating point bit level hacking
    i = 0x5f3759dfu32.wrapping_sub(i >> 1); // what the?
    y = f32::from_bits(i);

    // y = y * (threehalfs - (x2 * y * y));  — 1st iteration.
    // C groups the product as ((x2 * y) * y).
    y = fmul(y, fsub(threehalfs, fmul(fmul(x2, y), y)));

    y
}

/// `float Q_fabs( float f )` — clears the sign bit through an int alias.
#[allow(dead_code)]
pub fn q_fabs(f: f32) -> f32 {
    let tmp = f.to_bits() & 0x7FFF_FFFF;
    f32::from_bits(tmp)
}
