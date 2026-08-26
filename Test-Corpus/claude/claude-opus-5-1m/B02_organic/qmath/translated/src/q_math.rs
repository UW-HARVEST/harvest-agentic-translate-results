//! Translation of c_src/src/q_math.c (only the parts reachable from main) plus
//! the inline vector helpers from c_src/inc/q_shared.h.
//!
//! All arithmetic is performed in `f32` exactly like the C code (`vec_t` is
//! `float`), so results are bit-for-bit identical to the C original on targets
//! where `FLT_EVAL_METHOD == 0` (e.g. x86-64 SSE, aarch64).

/// `typedef vec_t vec3_t[3];`
pub type Vec3 = [f32; 3];

/// Turns a NaN into its quiet form, like the hardware does when it propagates a
/// NaN operand to the result.
#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `a + b` with the operand-order dependent NaN propagation of the generated
/// scalar SSE code (`addss`): when both operands are NaN the *first* one is
/// propagated, otherwise the NaN operand is. Written out explicitly because
/// LLVM is free to commute the operands of a plain `a + b`, which would change
/// which NaN payload/sign survives.
#[inline]
fn fadd(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a + b
    }
}

/// `a - b`, see [`fadd`].
#[inline]
fn fsub(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a - b
    }
}

/// `a * b`, see [`fadd`].
#[inline]
fn fmul(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a * b
    }
}

/// `#define DotProduct(x,y) ((x)[0]*(y)[0]+(x)[1]*(y)[1]+(x)[2]*(y)[2])`
#[inline]
pub fn dot_product(x: &Vec3, y: &Vec3) -> f32 {
    fadd(
        fadd(fmul(x[0], y[0]), fmul(x[1], y[1])),
        fmul(x[2], y[2]),
    )
}

/// ```c
/// float Q_rsqrt( float number )
/// {
///     uint32_t i;
///     float x2, y;
///     const float threehalfs = 1.5F;
///
///     x2 = number * 0.5F;
///     y  = number;
///     memcpy(&i, &y, sizeof(float));
///     i  = 0x5f3759dfu - (i >> 1);
///     memcpy(&y, &i, sizeof(float));
///     y  = y * (threehalfs - (x2 * y * y));
///     return y;
/// }
/// ```
pub fn q_rsqrt(number: f32) -> f32 {
    let mut i: u32;
    let x2: f32;
    let mut y: f32;
    let threehalfs: f32 = 1.5f32;

    x2 = fmul(number, 0.5f32);
    y = number;

    i = y.to_bits(); // evil floating point bit level hacking
    i = 0x5f3759dfu32.wrapping_sub(i >> 1); // what the?
    y = f32::from_bits(i);

    y = fmul(y, fsub(threehalfs, fmul(fmul(x2, y), y))); // 1st iteration

    y
}

/// ```c
/// static ID_INLINE void VectorNormalizeFast( vec3_t v )
/// {
///     float ilength;
///     ilength = Q_rsqrt( DotProduct( v, v ) );
///     v[0] *= ilength;
///     v[1] *= ilength;
///     v[2] *= ilength;
/// }
/// ```
pub fn vector_normalize_fast(v: &mut Vec3) {
    let ilength: f32 = q_rsqrt(dot_product(v, v));

    v[0] = fmul(v[0], ilength);
    v[1] = fmul(v[1], ilength);
    v[2] = fmul(v[2], ilength);
}
