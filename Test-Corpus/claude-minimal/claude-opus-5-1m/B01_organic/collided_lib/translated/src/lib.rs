#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::c_int;
use std::os::raw::c_void;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[inline]
pub fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[inline]
pub fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[inline]
pub fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

pub fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    if d2 < r2 {
        1
    } else {
        0
    }
}

pub fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 {
        1
    } else {
        0
    }
}

pub fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    let combined = d0 | d1 | d2 | d3;
    if combined == 0 {
        1
    } else {
        0
    }
}

/// # Safety
/// Caller must ensure `A` and `B` point to valid objects matching `typeA` and `typeB`.
#[no_mangle]
pub unsafe extern "C" fn collided(
    a: *const c_void,
    type_a: C2_TYPE,
    b: *const c_void,
    type_b: C2_TYPE,
) -> c_int {
    match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => {
                c2CircletoCircle(*(a as *const c2Circle), *(b as *const c2Circle))
            }
            C2_TYPE::C2_TYPE_AABB => {
                c2CircletoAABB(*(a as *const c2Circle), *(b as *const c2AABB))
            }
        },
        C2_TYPE::C2_TYPE_AABB => match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => {
                c2CircletoAABB(*(b as *const c2Circle), *(a as *const c2AABB))
            }
            C2_TYPE::C2_TYPE_AABB => {
                c2AABBtoAABB(*(a as *const c2AABB), *(b as *const c2AABB))
            }
        },
    }
}
