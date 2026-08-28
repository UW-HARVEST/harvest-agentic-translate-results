//! Port of the inline vector helpers from `c_src/inc/q_shared.h`.

use crate::q_math::q_rsqrt;
use crate::sse::{add, mul};

/// `typedef vec_t vec3_t[3];` with `typedef float vec_t;`
pub type Vec3 = [f32; 3];

/// ```c
/// #define DotProduct(x,y)  ((x)[0]*(y)[0]+(x)[1]*(y)[1]+(x)[2]*(y)[2])
/// ```
/// Summed strictly left to right, with the running total in the destination
/// operand of each `addss`, which is what the unoptimised C emits:
///
/// ```text
/// mulss %xmm0,%xmm1    # xmm1 = x[0]*y[0]
/// mulss %xmm2,%xmm0    # xmm0 = x[1]*y[1]
/// addss %xmm0,%xmm1    # xmm1 = (x[0]*y[0]) + (x[1]*y[1])
/// mulss %xmm2,%xmm0    # xmm0 = x[2]*y[2]
/// addss %xmm0,%xmm1    # xmm1 = ... + (x[2]*y[2])
/// ```
pub fn dot_product(x: &Vec3, y: &Vec3) -> f32 {
    let mut acc = mul(x[0], y[0]);
    acc = add(acc, mul(x[1], y[1]));
    acc = add(acc, mul(x[2], y[2]));
    acc
}

/// ```c
/// static ID_INLINE void VectorNormalizeFast( vec3_t v )
/// ```
/// Fast vector normalize routine that does not check to make sure that
/// length != 0, nor does it return length; uses the rsqrt approximation.
pub fn vector_normalize_fast(v: &mut Vec3) {
    let ilength: f32 = q_rsqrt(dot_product(v, v));

    // `v[i] *= ilength` keeps v[i] in the destination register.
    v[0] = mul(v[0], ilength);
    v[1] = mul(v[1], ilength);
    v[2] = mul(v[2], ilength);
}
