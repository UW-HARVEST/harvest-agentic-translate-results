use std::ffi::c_int;
use std::os::raw::c_void;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

// C2_TYPE enum values
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

#[inline]
fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[inline]
fn c2_maxv(a: C2v, b: C2v) -> C2v {
    // Match C ternary semantics: (a > b) ? a : b — when comparison is false
    // (including NaN cases), returns b.
    c2v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

#[inline]
fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> c_int {
    let d0: c_int = (b.max.x < a.min.x) as c_int;
    let d1: c_int = (a.max.x < b.min.x) as c_int;
    let d2: c_int = (b.max.y < a.min.y) as c_int;
    let d3: c_int = (a.max.y < b.min.y) as c_int;
    (!(d0 | d1 | d2 | d3) & 1) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    a: *const c_void,
    type_a: c_int,
    b: *const c_void,
    type_b: c_int,
) -> c_int {
    match type_a {
        x if x == C2_TYPE_CIRCLE => match type_b {
            y if y == C2_TYPE_CIRCLE => {
                c2_circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle))
            }
            y if y == C2_TYPE_AABB => {
                c2_circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB))
            }
            _ => 0,
        },
        x if x == C2_TYPE_AABB => match type_b {
            y if y == C2_TYPE_CIRCLE => {
                c2_circle_to_aabb(*(b as *const C2Circle), *(a as *const C2AABB))
            }
            y if y == C2_TYPE_AABB => {
                c2_aabb_to_aabb(*(a as *const C2AABB), *(b as *const C2AABB))
            }
            _ => 0,
        },
        _ => 0,
    }
}
