










use ::core::any::Any;

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
pub fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[no_mangle]
pub fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[no_mangle]
pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(a.x.max(b.x), a.y.max(b.y))
}

#[no_mangle]
pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(a.x.min(b.x), a.y.min(b.y))
}

#[no_mangle]
pub fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[no_mangle]
pub fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[no_mangle]
pub fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[no_mangle]
pub fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> i32 {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let r2 = (a.r + b.r) * (a.r + b.r);
    (d2 < r2) as i32
}

#[no_mangle]
pub fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> i32 {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as i32
}

#[no_mangle]
pub fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> i32 {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);

    let da = c2Dot(ap, n);
    let d2 = if da < 0.0 {
        c2Dot(ap, ap)
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            c2Dot(e, e)
        } else {
            let bp = c2Sub(A.p, B.b);
            c2Dot(bp, bp)
        }
    };

    let r = A.r + B.r;
    (d2 < r * r) as i32
}

#[no_mangle]
pub fn c2Collided(a: &c2Circle, b: &dyn ::core::any::Any, type_b: C2_TYPE) -> ::core::ffi::c_int {
    match type_b as ::core::ffi::c_uint {
        0 => b
            .downcast_ref::<c2Circle>()
            .map_or(0 as ::core::ffi::c_int, |circle| c2CircletoCircle(*a, *circle)),
        1 => b
            .downcast_ref::<c2AABB>()
            .map_or(0 as ::core::ffi::c_int, |aabb| c2CircletoAABB(*a, *aabb)),
        2 => b
            .downcast_ref::<c2Capsule>()
            .map_or(0 as ::core::ffi::c_int, |capsule| c2CircletoCapsule(*a, *capsule)),
        _ => 0 as ::core::ffi::c_int,
    }
}

#[no_mangle]
pub fn circle_collide(x: f32, y: f32, r: f32) -> i32 {
    let circle_in = c2Circle {
        p: c2V(x, y),
        r,
    };

    let circle = c2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = c2AABB {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };

    let capsule = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    let mut result = 0;
    result += c2Collided(&circle_in, &circle, C2_TYPE_CIRCLE);
    result += c2Collided(&circle_in, &aabb, C2_TYPE_AABB) << 1;
    result += c2Collided(&circle_in, &capsule, C2_TYPE_CAPSULE) << 2;
    result
}

