//! Rust translation of `c_src/src/lib.c`.
//!
//! Exposes the same C ABI: a `lm_vec2` struct of two `float`s and the
//! `to_barycentric` function that converts a point into the barycentric
//! coordinates of a triangle.
//!
//! # Bit-exact NaN propagation
//!
//! Plain `f32` operators in Rust are subject to operand commutation by the
//! optimizer, and on x86 SSE the *destination* register of a scalar FP
//! instruction decides which NaN payload survives when both operands are NaN.
//! That makes `a * b` and `b * a` observably different for NaN inputs even
//! though they are numerically identical otherwise.
//!
//! To stay byte-identical to the C build, every floating-point operation below
//! goes through an explicit `sse_*` helper that encodes the x86 scalar
//! semantics (destination operand wins, then source operand, otherwise the
//! hardware result). The destination/source assignment for each operation
//! mirrors the compiled C exactly - note that in `lm_dot2` the second product
//! and the final addition use their *right*-hand value as the destination,
//! which is why the helpers take operands in dest/src order rather than in
//! source order.

#![allow(non_camel_case_types)]

/// Quiet a NaN the way x86 does when propagating it: set the quiet bit while
/// preserving sign and payload.
#[inline]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `mulss dest, src` - result is `dest * src`, with `dest`'s NaN winning.
#[inline]
fn sse_mul(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet_nan(dest)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dest * src
    }
}

/// `addss dest, src` - result is `dest + src`, with `dest`'s NaN winning.
#[inline]
fn sse_add(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet_nan(dest)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dest + src
    }
}

/// `subss dest, src` - result is `dest - src`, with `dest`'s NaN winning.
#[inline]
fn sse_sub(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet_nan(dest)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dest - src
    }
}

/// `divss dest, src` - result is `dest / src`, with `dest`'s NaN winning.
#[inline]
fn sse_div(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet_nan(dest)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dest / src
    }
}

/// Mirrors `typedef struct lm_vec2 { float x, y; } lm_vec2;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

/// `static lm_vec2 lm_v2(float x, float y)`
#[inline]
fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

/// `static lm_vec2 lm_sub2(lm_vec2 a, lm_vec2 b)`
///
/// Both component subtractions keep the left operand as the destination, as in
/// the compiled C.
#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(sse_sub(a.x, b.x), sse_sub(a.y, b.y))
}

/// `static float lm_dot2(lm_vec2 a, lm_vec2 b)`
///
/// `a.x * b.x + a.y * b.y`. The compiled C evaluates the `y` product with
/// `b.y` as the destination register and then adds the `x` product *into* the
/// `y` product, so NaN preference is `b.y`, then `a.y`, then the `x` product.
#[inline]
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    let prod_x = sse_mul(a.x, b.x);
    let prod_y = sse_mul(b.y, a.y);
    sse_add(prod_y, prod_x)
}

/// `lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p)`
///
/// Note: the original C performs no degenerate-triangle check, so a zero
/// denominator yields infinities/NaNs. That behavior is reproduced as-is.
#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(
    p1: lm_vec2,
    p2: lm_vec2,
    p3: lm_vec2,
    p: lm_vec2,
) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);
    let inv_denom = sse_div(
        1.0f32,
        sse_sub(sse_mul(dot00, dot11), sse_mul(dot01, dot01)),
    );
    let u = sse_mul(
        sse_sub(sse_mul(dot11, dot02), sse_mul(dot01, dot12)),
        inv_denom,
    );
    let v = sse_mul(
        sse_sub(sse_mul(dot00, dot12), sse_mul(dot01, dot02)),
        inv_denom,
    );
    lm_v2(u, v)
}
