use std::ffi::c_int;

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
struct C2AABB {
    min: C2v,
    max: C2v,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
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
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v {
    c2v(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2_maxv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2_minv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_clampv(a, lo, hi)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: C2v, b: C2v) -> C2v {
    c2_sub(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 {
    c2_dot(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int {
    c2_circle_to_circle(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2AABB) -> c_int {
    c2_circle_to_aabb(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2AABB, b: C2AABB) -> c_int {
    c2_aabb_to_aabb(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    a: *const std::ffi::c_void,
    type_a: c_int,
    b: *const std::ffi::c_void,
    type_b: c_int,
) -> c_int {
    match type_a {
        C2_TYPE_CIRCLE => match type_b {
            C2_TYPE_CIRCLE => {
                c2_circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle))
            }
            C2_TYPE_AABB => c2_circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB)),
            _ => 0,
        },
        C2_TYPE_AABB => match type_b {
            C2_TYPE_CIRCLE => c2_circle_to_aabb(*(b as *const C2Circle), *(a as *const C2AABB)),
            C2_TYPE_AABB => c2_aabb_to_aabb(*(a as *const C2AABB), *(b as *const C2AABB)),
            _ => 0,
        },
        _ => 0,
    }
}
