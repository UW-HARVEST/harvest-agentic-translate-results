use std::ffi::c_int;
use std::os::raw::c_void;

#[allow(non_camel_case_types)]
pub type C2_TYPE = c_int;

pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Copy, Clone)]
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
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
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

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
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

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int {
    let d0: c_int = if b.max.x < a.min.x { 1 } else { 0 };
    let d1: c_int = if a.max.x < b.min.x { 1 } else { 0 };
    let d2: c_int = if b.max.y < a.min.y { 1 } else { 0 };
    let d3: c_int = if a.max.y < b.min.y { 1 } else { 0 };
    let combined = d0 | d1 | d2 | d3;
    if combined == 0 {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    a: *const c_void,
    type_a: C2_TYPE,
    b: *const c_void,
    type_b: C2_TYPE,
) -> c_int {
    match type_a {
        x if x == C2_TYPE_CIRCLE => match type_b {
            y if y == C2_TYPE_CIRCLE => {
                c2CircletoCircle(*(a as *const c2Circle), *(b as *const c2Circle))
            }
            y if y == C2_TYPE_AABB => {
                c2CircletoAABB(*(a as *const c2Circle), *(b as *const c2AABB))
            }
            _ => 0,
        },
        x if x == C2_TYPE_AABB => match type_b {
            y if y == C2_TYPE_CIRCLE => {
                c2CircletoAABB(*(b as *const c2Circle), *(a as *const c2AABB))
            }
            y if y == C2_TYPE_AABB => {
                c2AABBtoAABB(*(a as *const c2AABB), *(b as *const c2AABB))
            }
            _ => 0,
        },
        _ => 0,
    }
}
