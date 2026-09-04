//! Translation of the inline vector helpers from `q_shared.h`.

use crate::fops::{fadd, fmul};
use crate::q_math::q_rsqrt;

/// `vec_t` is `float`; `vec3_t` is `vec_t[3]`.
pub type Vec3 = [f32; 3];

/// `#define DotProduct(x,y) ((x)[0]*(y)[0]+(x)[1]*(y)[1]+(x)[2]*(y)[2])`
///
/// The macro expands to a left-associated sum, and the operand order of each
/// `+` and `*` is significant for NaN propagation, hence `fadd`/`fmul`.
pub fn dot_product(x: &Vec3, y: &Vec3) -> f32 {
    fadd(fadd(fmul(x[0], y[0]), fmul(x[1], y[1])), fmul(x[2], y[2]))
}

/// Fast vector normalize routine that does not check to make sure that
/// length != 0, nor does it return length; uses the rsqrt approximation.
pub fn vector_normalize_fast(v: &mut Vec3) {
    let ilength: f32 = q_rsqrt(dot_product(v, v));

    // `v[i] *= ilength` — `v[i]` is the destination operand.
    v[0] = fmul(v[0], ilength);
    v[1] = fmul(v[1], ilength);
    v[2] = fmul(v[2], ilength);
}
