//! Rust translation of the C collision library in `c_src/`.
//!
//! The C library compiles `src/lib.c` into a shared object that exports every
//! non-static function it defines. This crate reproduces that exact public ABI:
//! `c2V`, `c2Maxv`, `c2Minv`, `c2Clampv`, `c2Sub`, `c2Dot`, `c2CircletoCircle`,
//! `c2CircletoAABB`, `c2AABBtoAABB` and `collided`.
//!
//! Behaviour is preserved bit-for-bit, including the exact ternary comparison
//! semantics used by `c2Maxv`/`c2Minv` (which differ from `f32::max`/`f32::min`
//! for NaN operands) and the `int`-based bitwise logic of `c2AABBtoAABB`.

#![allow(non_snake_case)]

use std::ffi::c_int;

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB } C2_TYPE;`
///
/// A C enum with these enumerators is `unsigned int`-compatible and is passed
/// as a 4-byte integer, so the FFI signature uses `c_int` (matching what the C
/// compiler emits for the `collided` entry point on the SysV ABI).
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// The x86 "QNaN floating-point indefinite" value, produced by SSE scalar
/// arithmetic whenever an invalid operation (`0 * inf`, `inf - inf`) occurs.
const QNAN_INDEFINITE: u32 = 0xFFC0_0000;

/// Quiet a NaN the way x86 does: set the most significant mantissa bit and
/// leave the sign and the remaining payload untouched.
#[inline]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// Emulates `mulss dst, src` (result written back to `dst`).
///
/// SSE two-operand scalar arithmetic resolves NaN operands by preferring the
/// *destination* operand: if `dst` is a NaN the result is `dst` quieted,
/// otherwise if `src` is a NaN the result is `src` quieted. This matters
/// because the two products in [`c2Dot`] can both be NaN with different
/// payloads, and the C compiler pins a specific operand order.
#[inline]
fn mulss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        return quiet_nan(dst);
    }
    if src.is_nan() {
        return quiet_nan(src);
    }
    if (dst == 0.0 && src.is_infinite()) || (dst.is_infinite() && src == 0.0) {
        return f32::from_bits(QNAN_INDEFINITE);
    }
    dst * src
}

/// Emulates `addss dst, src`. See [`mulss`] for the NaN-priority rules.
#[inline]
fn addss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        return quiet_nan(dst);
    }
    if src.is_nan() {
        return quiet_nan(src);
    }
    if dst.is_infinite() && src.is_infinite() && dst.is_sign_negative() != src.is_sign_negative() {
        return f32::from_bits(QNAN_INDEFINITE);
    }
    dst + src
}

/// Emulates `subss dst, src`, i.e. `dst - src`. See [`mulss`].
#[inline]
fn subss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        return quiet_nan(dst);
    }
    if src.is_nan() {
        return quiet_nan(src);
    }
    if dst.is_infinite() && src.is_infinite() && dst.is_sign_negative() == src.is_sign_negative() {
        return f32::from_bits(QNAN_INDEFINITE);
    }
    dst - src
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

/// Uses `a > b ? a : b` rather than `f32::max`: when either operand is NaN the
/// comparison is false and the second operand is returned, matching the C.
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

/// Uses `a < b ? a : b` rather than `f32::min`, for the same NaN reason as
/// [`c2Maxv`].
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = subss(a.x, b.x);
    a.y = subss(a.y, b.y);
    a
}

/// `a.x * b.x + a.y * b.y`.
///
/// The C compiler emits the second product as `mulss dst=b.y, src=a.y` and the
/// sum as `addss dst=(a.y*b.y), src=(a.x*b.x)`. That operand order is
/// reproduced here so the NaN payload returned for NaN/infinite inputs is
/// bit-identical to the C build.
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    let p = mulss(a.x, b.x);
    let q = mulss(b.y, a.y);
    addss(q, p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = addss(B.r, A.r);
    r2 = mulss(r2, r2);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = mulss(A.r, A.r);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// `int collided(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB);`
///
/// The pointers are dereferenced according to the supplied type tags, exactly
/// as the C does — including the AABB/CIRCLE case, which reinterprets `B` as
/// the circle and `A` as the box. Unknown type tags yield `0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    A: *const core::ffi::c_void,
    typeA: c_int,
    B: *const core::ffi::c_void,
    typeB: c_int,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoCircle(unsafe { *(A as *const c2Circle) }, unsafe {
                    *(B as *const c2Circle)
                })
            }
            C2_TYPE_AABB => c2CircletoAABB(unsafe { *(A as *const c2Circle) }, unsafe {
                *(B as *const c2AABB)
            }),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(unsafe { *(B as *const c2Circle) }, unsafe {
                *(A as *const c2AABB)
            }),
            C2_TYPE_AABB => {
                c2AABBtoAABB(unsafe { *(A as *const c2AABB) }, unsafe { *(B as *const c2AABB) })
            }
            _ => 0,
        },
        _ => 0,
    }
}
