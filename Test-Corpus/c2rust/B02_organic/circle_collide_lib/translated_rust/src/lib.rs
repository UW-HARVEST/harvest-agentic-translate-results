pub type C2_TYPE = ::core::ffi::c_uint;
pub const C2_TYPE_CAPSULE: C2_TYPE = 2;
pub const C2_TYPE_AABB: C2_TYPE = 1;
pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: ::core::ffi::c_float,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}
#[no_mangle]
pub unsafe extern "C" fn c2V(mut x: ::core::ffi::c_float, mut y: ::core::ffi::c_float) -> c2v {
    let mut a: c2v = c2v { x: 0., y: 0. };
    a.x = x;
    a.y = y;
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn c2Mulvs(mut a: c2v, mut b: ::core::ffi::c_float) -> c2v {
    a.x *= b;
    a.y *= b;
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
pub unsafe extern "C" fn c2CircletoCapsule(
    mut A: c2Circle,
    mut B: c2Capsule,
) -> ::core::ffi::c_int {
    let mut n: c2v = c2Sub(B.b, B.a);
    let mut ap: c2v = c2Sub(A.p, B.a);
    let mut da: ::core::ffi::c_float = c2Dot(ap, n);
    let mut d2: ::core::ffi::c_float = 0.;
    if da < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        d2 = c2Dot(ap, ap);
    } else {
        let mut db: ::core::ffi::c_float = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
            let mut e: c2v = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let mut bp: c2v = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let mut r: ::core::ffi::c_float = A.r + B.r;
    return (d2 < r * r) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2Collided(
    mut A: *const ::core::ffi::c_void,
    mut B: *const ::core::ffi::c_void,
    mut typeB: C2_TYPE,
) -> ::core::ffi::c_int {
    match typeB as ::core::ffi::c_uint {
        0 => return c2CircletoCircle(*(A as *mut c2Circle), *(B as *mut c2Circle)),
        1 => return c2CircletoAABB(*(A as *mut c2Circle), *(B as *mut c2AABB)),
        2 => return c2CircletoCapsule(*(A as *mut c2Circle), *(B as *mut c2Capsule)),
        _ => return 0 as ::core::ffi::c_int,
    };
}
#[no_mangle]
pub unsafe extern "C" fn circle_collide(
    mut x: ::core::ffi::c_float,
    mut y: ::core::ffi::c_float,
    mut r: ::core::ffi::c_float,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut circle_in: c2Circle = c2Circle {
        p: c2v { x: 0., y: 0. },
        r: 0.,
    };
    circle_in.p = c2V(x, y);
    circle_in.r = r;
    let mut circle: c2Circle = c2Circle {
        p: c2v { x: 0., y: 0. },
        r: 0.,
    };
    circle.p = c2V(-70.0f32, 0 as ::core::ffi::c_int as ::core::ffi::c_float);
    circle.r = 20.0f32;
    let mut aabb: c2AABB = c2AABB {
        min: c2v { x: 0., y: 0. },
        max: c2v { x: 0., y: 0. },
    };
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);
    let mut capsule: c2Capsule = c2Capsule {
        a: c2v { x: 0., y: 0. },
        b: c2v { x: 0., y: 0. },
        r: 0.,
    };
    capsule.a = c2V(-40.0f32, 40.0f32);
    capsule.b = c2V(-20.0f32, 100.0f32);
    capsule.r = 10.0f32;
    result += c2Collided(
        &raw mut circle_in as *const ::core::ffi::c_void,
        &raw mut circle as *const ::core::ffi::c_void,
        C2_TYPE_CIRCLE,
    );
    result += c2Collided(
        &raw mut circle_in as *const ::core::ffi::c_void,
        &raw mut aabb as *const ::core::ffi::c_void,
        C2_TYPE_AABB,
    ) << 1 as ::core::ffi::c_int;
    result += c2Collided(
        &raw mut circle_in as *const ::core::ffi::c_void,
        &raw mut capsule as *const ::core::ffi::c_void,
        C2_TYPE_CAPSULE,
    ) << 2 as ::core::ffi::c_int;
    return result;
}
