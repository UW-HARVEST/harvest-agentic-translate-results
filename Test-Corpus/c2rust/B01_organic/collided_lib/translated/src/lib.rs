pub type C2_TYPE = ::core::ffi::c_uint;
pub const C2_TYPE_AABB: C2_TYPE = 1;
pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2v {
    pub x: ::core::ffi::c_float,
    pub y: ::core::ffi::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Circle {
    pub p: c2v,
    pub r: ::core::ffi::c_float,
}
#[no_mangle]
pub unsafe extern "C" fn c2V(mut x: ::core::ffi::c_float, mut y: ::core::ffi::c_float) -> c2v {
    let mut a: c2v = c2v { x: 0., y: 0. };
    a.x = x;
    a.y = y;
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn c2Maxv(mut a: c2v, mut b: c2v) -> c2v {
    return c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    );
}
#[no_mangle]
pub unsafe extern "C" fn c2Minv(mut a: c2v, mut b: c2v) -> c2v {
    return c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    );
}
#[no_mangle]
pub unsafe extern "C" fn c2Clampv(mut a: c2v, mut lo: c2v, mut hi: c2v) -> c2v {
    return c2Maxv(lo, c2Minv(a, hi));
}
#[no_mangle]
pub unsafe extern "C" fn c2Sub(mut a: c2v, mut b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn c2Dot(mut a: c2v, mut b: c2v) -> ::core::ffi::c_float {
    return a.x * b.x + a.y * b.y;
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoCircle(mut A: c2Circle, mut B: c2Circle) -> ::core::ffi::c_int {
    let mut c: c2v = c2Sub(B.p, A.p);
    let mut d2: ::core::ffi::c_float = c2Dot(c, c);
    let mut r2: ::core::ffi::c_float = A.r + B.r;
    r2 = r2 * r2;
    return (d2 < r2) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoAABB(mut A: c2Circle, mut B: c2AABB) -> ::core::ffi::c_int {
    let mut L: c2v = c2Clampv(A.p, B.min, B.max);
    let mut ab: c2v = c2Sub(A.p, L);
    let mut d2: ::core::ffi::c_float = c2Dot(ab, ab);
    let mut r2: ::core::ffi::c_float = A.r * A.r;
    return (d2 < r2) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2AABBtoAABB(mut A: c2AABB, mut B: c2AABB) -> ::core::ffi::c_int {
    let mut d0: ::core::ffi::c_int = (B.max.x < A.min.x) as ::core::ffi::c_int;
    let mut d1: ::core::ffi::c_int = (A.max.x < B.min.x) as ::core::ffi::c_int;
    let mut d2: ::core::ffi::c_int = (B.max.y < A.min.y) as ::core::ffi::c_int;
    let mut d3: ::core::ffi::c_int = (A.max.y < B.min.y) as ::core::ffi::c_int;
    return (d0 | d1 | d2 | d3 == 0) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn collided(
    mut A: *const ::core::ffi::c_void,
    mut typeA: C2_TYPE,
    mut B: *const ::core::ffi::c_void,
    mut typeB: C2_TYPE,
) -> ::core::ffi::c_int {
    match typeA as ::core::ffi::c_uint {
        0 => match typeB as ::core::ffi::c_uint {
            0 => {
                return c2CircletoCircle(*(A as *mut c2Circle), *(B as *mut c2Circle));
            }
            1 => return c2CircletoAABB(*(A as *mut c2Circle), *(B as *mut c2AABB)),
            _ => return 0 as ::core::ffi::c_int,
        },
        1 => match typeB as ::core::ffi::c_uint {
            0 => return c2CircletoAABB(*(B as *mut c2Circle), *(A as *mut c2AABB)),
            1 => return c2AABBtoAABB(*(A as *mut c2AABB), *(B as *mut c2AABB)),
            _ => return 0 as ::core::ffi::c_int,
        },
        _ => return 0 as ::core::ffi::c_int,
    };
}
