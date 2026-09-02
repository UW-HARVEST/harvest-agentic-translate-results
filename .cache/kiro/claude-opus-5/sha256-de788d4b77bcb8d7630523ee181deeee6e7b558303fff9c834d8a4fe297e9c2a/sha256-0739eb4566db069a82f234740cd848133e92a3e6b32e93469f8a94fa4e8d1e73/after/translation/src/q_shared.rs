//! Translation of the inline vector helpers from `q_shared.h`.

use crate::q_math::q_rsqrt;

/// `vec_t` is `float`; `vec3_t` is `vec_t[3]`.
pub type Vec3 = [f32; 3];

/// `#define DotProduct(x,y) ((x)[0]*(y)[0]+(x)[1]*(y)[1]+(x)[2]*(y)[2])`
pub fn dot_product(x: &Vec3, y: &Vec3) -> f32 {
    x[0] * y[0] + x[1] * y[1] + x[2] * y[2]
}

/// Fast vector normalize routine that does not check to make sure that
/// length != 0, nor does it return length; uses the rsqrt approximation.
pub fn vector_normalize_fast(v: &mut Vec3) {
    let ilength: f32 = q_rsqrt(dot_product(v, v));

    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}
