//! Rust translation of c_src/src/lib.c (tinyc2-style 2D collision tests).
//!
//! Behaviour is preserved exactly, including the float comparison semantics
//! (`a > b ? a : b` rather than `fmax`, which differ for NaN) and the
//! `default: return 0;` fallbacks for out-of-range `C2_TYPE` values.

#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_int;
use std::ffi::c_void;

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB } C2_TYPE;`
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    // Deliberately not f32::max: the C ternary yields `b` when the
    // comparison is false (e.g. when either operand is NaN).
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

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
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let r2 = A.r + B.r;
    let r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
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

/// # Safety
///
/// `A` and `B` must point to properly aligned, initialised objects of the
/// type indicated by `typeA` / `typeB`, exactly as the C version requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
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
            // Note: arguments are taken from B then A here, matching the C.
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
