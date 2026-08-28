//! Rust translation of `c_src/src/lib.c`.
//!
//! Exposes the same C ABI: a `lm_vec2` struct of two `float`s and the
//! `to_barycentric` function that converts a point into the barycentric
//! coordinates of a triangle.

#![allow(non_camel_case_types)]

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
#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(a.x - b.x, a.y - b.y)
}

/// `static float lm_dot2(lm_vec2 a, lm_vec2 b)`
#[inline]
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    a.x * b.x + a.y * b.y
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
    let inv_denom = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_v2(u, v)
}
