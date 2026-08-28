//! Port of the parts of `q_math.c` reachable from the driver.

use crate::fpu;

/// `float Q_rsqrt( float number )` -- the famous fast inverse square root.
///
/// The `memcpy()` type punning of the original is expressed with
/// `to_bits`/`from_bits`, which is the exact same bit reinterpretation.
pub fn q_rsqrt(number: f32) -> f32 {
    let i: u32;
    let x2: f32;
    let mut y: f32;
    const THREEHALFS: f32 = 1.5;

    x2 = fpu::mul(number, 0.5);
    y = number;

    i = y.to_bits(); // evil floating point bit level hacking
    let i = 0x5f3759dfu32.wrapping_sub(i >> 1); // what the?
    y = f32::from_bits(i);

    // y  = y * (threehalfs - (x2 * y * y));   // 1st iteration
    y = fpu::mul(y, fpu::sub(THREEHALFS, fpu::mul(fpu::mul(x2, y), y)));
    //	y  = y * ( threehalfs - ( x2 * y * y ) );   // 2nd iteration, this can be removed

    y
}
