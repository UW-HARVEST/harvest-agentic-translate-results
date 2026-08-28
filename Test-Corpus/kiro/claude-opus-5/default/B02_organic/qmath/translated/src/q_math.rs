//! Port of the routines from `c_src/src/q_math.c` that the driver needs.

/// ```c
/// float Q_rsqrt( float number )
/// ```
/// The Quake III fast inverse square root, including its single Newton
/// iteration and the `0x5f3759df` magic constant.
pub fn q_rsqrt(number: f32) -> f32 {
    let threehalfs: f32 = 1.5;

    let x2: f32 = number * 0.5;
    let mut y: f32 = number;

    // memcpy(&i, &y, sizeof(float)) — evil floating point bit level hacking
    let mut i: u32 = y.to_bits();
    i = 0x5f3759dfu32.wrapping_sub(i >> 1); // what the?
    y = f32::from_bits(i);

    // 1st iteration; the commented-out 2nd iteration of the original stays out.
    y = y * (threehalfs - (x2 * y * y));

    y
}

/// ```c
/// float Q_fabs( float f )
/// ```
/// Kept for parity with the C translation unit even though `main` never calls
/// it: it clears the sign bit through a type-punned integer.
#[allow(dead_code)]
pub fn q_fabs(f: f32) -> f32 {
    let tmp = f.to_bits() & 0x7FFF_FFFF;
    f32::from_bits(tmp)
}
