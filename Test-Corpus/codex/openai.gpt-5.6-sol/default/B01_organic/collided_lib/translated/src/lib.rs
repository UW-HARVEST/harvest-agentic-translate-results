use std::ffi::{c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    c_int::from(d2 < r2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    c_int::from(d2 < r2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = c_int::from(b.max.x < a.min.x);
    let d1 = c_int::from(a.max.x < b.min.x);
    let d2 = c_int::from(b.max.y < a.min.y);
    let d3 = c_int::from(a.max.y < b.min.y);
    c_int::from((d0 | d1 | d2 | d3) == 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    a: *const c_void,
    type_a: c_int,
    b: *const c_void,
    type_b: c_int,
) -> c_int {
    match type_a {
        C2_TYPE_CIRCLE => match type_b {
            C2_TYPE_CIRCLE => {
                // SAFETY: This matches the C API's requirement that both pointers
                // reference valid c2Circle values for this type combination.
                unsafe { c2CircletoCircle(*(a.cast::<C2Circle>()), *(b.cast::<C2Circle>())) }
            }
            C2_TYPE_AABB => {
                // SAFETY: This matches the corresponding C pointer contract.
                unsafe { c2CircletoAABB(*(a.cast::<C2Circle>()), *(b.cast::<C2Aabb>())) }
            }
            _ => 0,
        },
        C2_TYPE_AABB => match type_b {
            C2_TYPE_CIRCLE => {
                // SAFETY: This matches the corresponding C pointer contract.
                unsafe { c2CircletoAABB(*(b.cast::<C2Circle>()), *(a.cast::<C2Aabb>())) }
            }
            C2_TYPE_AABB => {
                // SAFETY: This matches the corresponding C pointer contract.
                unsafe { c2AABBtoAABB(*(a.cast::<C2Aabb>()), *(b.cast::<C2Aabb>())) }
            }
            _ => 0,
        },
        _ => 0,
    }
}
