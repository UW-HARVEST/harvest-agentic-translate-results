use std::ffi::{c_int, c_void};
use std::ptr;

#[allow(non_camel_case_types)]
pub type C2_TYPE = c_int;

const C2_TYPE_CIRCLE: C2_TYPE = 0;
const C2_TYPE_AABB: C2_TYPE = 1;

#[repr(C)]
#[derive(Clone, Copy)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

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

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_aabb_to_aabb(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    a: *const c_void,
    type_a: C2_TYPE,
    b: *const c_void,
    type_b: C2_TYPE,
) -> c_int {
    match type_a {
        C2_TYPE_CIRCLE => match type_b {
            C2_TYPE_CIRCLE => c2_circle_to_circle(
                ptr::read(a.cast::<C2Circle>()),
                ptr::read(b.cast::<C2Circle>()),
            ),
            C2_TYPE_AABB => c2_circle_to_aabb(
                ptr::read(a.cast::<C2Circle>()),
                ptr::read(b.cast::<C2Aabb>()),
            ),
            _ => 0,
        },
        C2_TYPE_AABB => match type_b {
            C2_TYPE_CIRCLE => c2_circle_to_aabb(
                ptr::read(b.cast::<C2Circle>()),
                ptr::read(a.cast::<C2Aabb>()),
            ),
            C2_TYPE_AABB => {
                c2_aabb_to_aabb(ptr::read(a.cast::<C2Aabb>()), ptr::read(b.cast::<C2Aabb>()))
            }
            _ => 0,
        },
        _ => 0,
    }
}
