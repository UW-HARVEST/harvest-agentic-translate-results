use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let r2 = A.r + B.r;
    let r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = B.max.x < A.min.x;
    let d1 = A.max.x < B.min.x;
    let d2 = B.max.y < A.min.y;
    let d3 = A.max.y < B.min.y;
    (!(d0 | d1 | d2 | d3)) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn collided(A: *const c_void, typeA: C2_TYPE, B: *const c_void, typeB: C2_TYPE) -> c_int {
    unsafe {
        match typeA {
            C2_TYPE::C2_TYPE_CIRCLE => {
                match typeB {
                    C2_TYPE::C2_TYPE_CIRCLE => {
                        c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle))
                    }
                    C2_TYPE::C2_TYPE_AABB => {
                        c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB))
                    }
                    _ => 0,
                }
            }
            C2_TYPE::C2_TYPE_AABB => {
                match typeB {
                    C2_TYPE::C2_TYPE_CIRCLE => {
                        c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB))
                    }
                    C2_TYPE::C2_TYPE_AABB => {
                        c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB))
                    }
                    _ => 0,
                }
            }
            _ => 0,
        }
    }
}
