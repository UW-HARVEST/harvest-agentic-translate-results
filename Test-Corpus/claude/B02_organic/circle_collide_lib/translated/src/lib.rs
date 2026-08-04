use std::ffi::c_int;
use std::os::raw::c_void;

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[allow(non_snake_case)]
fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[allow(non_snake_case)]
fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[allow(non_snake_case)]
fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[allow(non_snake_case)]
fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[allow(non_snake_case)]
fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[allow(non_snake_case)]
fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[allow(non_snake_case)]
fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[allow(non_snake_case)]
fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[allow(non_snake_case)]
fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

#[allow(non_snake_case)]
fn c2CircletoCapsule(a: c2Circle, b: c2Capsule) -> c_int {
    let n = c2Sub(b.b, b.a);
    let ap = c2Sub(a.p, b.a);
    let da = c2Dot(ap, n);
    let d2: f32;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(a.p, b.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = a.r + b.r;
    (d2 < r * r) as c_int
}

#[allow(non_snake_case)]
fn c2Collided(a: *const c_void, b: *const c_void, type_b: C2_TYPE) -> c_int {
    unsafe {
        match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => {
                c2CircletoCircle(*(a as *const c2Circle), *(b as *const c2Circle))
            }
            C2_TYPE::C2_TYPE_AABB => {
                c2CircletoAABB(*(a as *const c2Circle), *(b as *const c2AABB))
            }
            C2_TYPE::C2_TYPE_CAPSULE => {
                c2CircletoCapsule(*(a as *const c2Circle), *(b as *const c2Capsule))
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let circle_in = c2Circle {
        p: c2V(x, y),
        r,
    };

    let circle = c2Circle {
        p: c2V(-70.0f32, 0.0),
        r: 20.0f32,
    };

    let aabb = c2AABB {
        min: c2V(-40.0f32, -40.0f32),
        max: c2V(-15.0f32, -15.0f32),
    };

    let capsule = c2Capsule {
        a: c2V(-40.0f32, 40.0f32),
        b: c2V(-20.0f32, 100.0f32),
        r: 10.0f32,
    };

    result += c2Collided(
        &circle_in as *const _ as *const c_void,
        &circle as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CIRCLE,
    );

    result += c2Collided(
        &circle_in as *const _ as *const c_void,
        &aabb as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_AABB,
    ) << 1;

    result += c2Collided(
        &circle_in as *const _ as *const c_void,
        &capsule as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE,
    ) << 2;

    result
}
