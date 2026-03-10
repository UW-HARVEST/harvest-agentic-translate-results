use std::os::raw::c_int;

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

#[repr(C)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
}

fn c2_v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2_maxv(a: c2v, b: c2v) -> c2v {
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_minv(a: c2v, b: c2v) -> c2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_aabb_to_aabb(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn collided(
    a: *const std::ffi::c_void,
    type_a: C2_TYPE,
    b: *const std::ffi::c_void,
    type_b: C2_TYPE,
) -> c_int {
    match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => {
                let a = unsafe { *(a as *const c2Circle) };
                let b = unsafe { *(b as *const c2Circle) };
                c2_circle_to_circle(a, b)
            }
            C2_TYPE::C2_TYPE_AABB => {
                let a = unsafe { *(a as *const c2Circle) };
                let b = unsafe { *(b as *const c2AABB) };
                c2_circle_to_aabb(a, b)
            }
        },
        C2_TYPE::C2_TYPE_AABB => match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => {
                let b_circle = unsafe { *(b as *const c2Circle) };
                let a_aabb = unsafe { *(a as *const c2AABB) };
                c2_circle_to_aabb(b_circle, a_aabb)
            }
            C2_TYPE::C2_TYPE_AABB => {
                let a = unsafe { *(a as *const c2AABB) };
                let b = unsafe { *(b as *const c2AABB) };
                c2_aabb_to_aabb(a, b)
            }
        },
    }
}
