use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(a.x.max(b.x), a.y.max(b.y))
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(a.x.min(b.x), a.y.min(b.y))
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(a: C2v, b: C2v) -> C2v {
    c2_v(a.x - b.x, a.y - b.y)
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> bool {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = (a.r + b.r) * (a.r + b.r);
    d2 < r2
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> bool {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    d2 < r2
}

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> bool {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    !(d0 || d1 || d2 || d3)
}

#[unsafe(no_mangle)]
pub extern "C" fn collided(a: *const c_void, type_a: C2_TYPE, b: *const c_void, type_b: C2_TYPE) -> c_int {
    if a.is_null() || b.is_null() {
        return 0;
    }

    match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => {
                let a_val = unsafe { *(a as *const C2Circle) };
                let b_val = unsafe { *(b as *const C2Circle) };
                c2_circle_to_circle(a_val, b_val) as c_int
            }
            C2_TYPE::C2_TYPE_AABB => {
                let a_val = unsafe { *(a as *const C2Circle) };
                let b_val = unsafe { *(b as *const C2AABB) };
                c2_circle_to_aabb(a_val, b_val) as c_int
            }
        },
        C2_TYPE::C2_TYPE_AABB => match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => {
                let b_val = unsafe { *(b as *const C2Circle) };
                let a_val = unsafe { *(a as *const C2AABB) };
                c2_circle_to_aabb(b_val, a_val) as c_int
            }
            C2_TYPE::C2_TYPE_AABB => {
                let a_val = unsafe { *(a as *const C2AABB) };
                let b_val = unsafe { *(b as *const C2AABB) };
                c2_aabb_to_aabb(a_val, b_val) as c_int
            }
        },
    }
}
