








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
pub fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[no_mangle]
pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(a.x.max(b.x), a.y.max(b.y))
}

#[no_mangle]
pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x.min(b.x),
        y: a.y.min(b.y),
    }
}

#[no_mangle]
pub fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[no_mangle]
pub fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[no_mangle]
pub fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[no_mangle]
pub fn c2CircletoCircle(a: &c2Circle, b: &c2Circle) -> bool {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let r2 = (a.r + b.r) * (a.r + b.r);
    d2 < r2
}

#[no_mangle]
pub fn c2CircletoAABB(a: &c2Circle, b: &c2AABB) -> bool {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    d2 < r2
}

#[no_mangle]
pub fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> bool {
    !(b.max.x < a.min.x || a.max.x < b.min.x || b.max.y < a.min.y || a.max.y < b.min.y)
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
            let a = &*(A as *const c2Circle);
            let b = &*(B as *const c2Circle);
            return c2CircletoCircle(a, b) as ::core::ffi::c_int;
        }
        1 => {
            let a = &*(A as *const c2Circle);
            let b = &*(B as *const c2AABB);
            return c2CircletoAABB(a, b) as ::core::ffi::c_int;
        }
        _ => return 0 as ::core::ffi::c_int,
    },
    1 => match typeB as ::core::ffi::c_uint {
        0 => {
            let a = &*(B as *const c2Circle);
            let b = &*(A as *const c2AABB);
            return c2CircletoAABB(a, b) as ::core::ffi::c_int;
        }
        1 => return c2AABBtoAABB(*(A as *mut c2AABB), *(B as *mut c2AABB)) as ::core::ffi::c_int,
        _ => return 0 as ::core::ffi::c_int,
    },
    _ => return 0 as ::core::ffi::c_int,
};
    match typeA as ::core::ffi::c_uint {
    0 => match typeB as ::core::ffi::c_uint {
        0 => {
            let a = &*(A as *const c2Circle);
            let b = &*(B as *const c2Circle);
            return c2CircletoCircle(a, b) as ::core::ffi::c_int;
        }
        1 => {
            let a = &*(A as *const c2Circle);
            let b = &*(B as *const c2AABB);
            return c2CircletoAABB(a, b) as ::core::ffi::c_int;
        }
        _ => return 0 as ::core::ffi::c_int,
    },
    1 => match typeB as ::core::ffi::c_uint {
        0 => {
            let a = &*(B as *const c2Circle);
            let b = &*(A as *const c2AABB);
            return c2CircletoAABB(a, b) as ::core::ffi::c_int;
        }
        1 => return c2AABBtoAABB(*(A as *mut c2AABB), *(B as *mut c2AABB)) as ::core::ffi::c_int,
        _ => return 0 as ::core::ffi::c_int,
    },
    _ => return 0 as ::core::ffi::c_int,
};
}
