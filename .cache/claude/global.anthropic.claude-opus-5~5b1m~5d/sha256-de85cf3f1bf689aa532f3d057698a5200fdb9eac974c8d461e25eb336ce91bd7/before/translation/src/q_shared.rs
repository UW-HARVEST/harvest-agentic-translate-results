//! Port of the inline helpers from `q_shared.h` used by the driver.

use crate::fpu;
use crate::q_math::q_rsqrt;

/// `vec3_t` -- three C `float`s.
pub type Vec3 = [f32; 3];

/// `#define DotProduct(x,y) ((x)[0]*(y)[0]+(x)[1]*(y)[1]+(x)[2]*(y)[2])`
///
/// Evaluated left to right in single precision, exactly like the C macro.
pub fn dot_product(x: &Vec3, y: &Vec3) -> f32 {
    fpu::add(
        fpu::add(fpu::mul(x[0], y[0]), fpu::mul(x[1], y[1])),
        fpu::mul(x[2], y[2]),
    )
}

/// fast vector normalize routine that does not check to make sure
/// that length != 0, nor does it return length, uses rsqrt approximation
pub fn vector_normalize_fast(v: &mut Vec3) {
    let ilength: f32 = q_rsqrt(dot_product(v, v));

    v[0] = fpu::mul(v[0], ilength);
    v[1] = fpu::mul(v[1], ilength);
    v[2] = fpu::mul(v[2], ilength);
}
