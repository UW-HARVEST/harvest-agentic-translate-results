use std::os::raw::c_int;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(a: c2Circle, b: c2Capsule) -> c_int {
    let n = c2Sub(b.b, b.a);
    let ap = c2Sub(a.p, b.a);
    let da = c2Dot(ap, n);
    let d2;
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

#[unsafe(no_mangle)]
pub extern "C" fn c2Collided(a: *const u8, b: *const u8, type_b: c_int) -> c_int {
    unsafe {
        match type_b {
            0 => c2CircletoCircle(*(a as *const c2Circle), *(b as *const c2Circle)),
            1 => c2CircletoAABB(*(a as *const c2Circle), *(b as *const c2AABB)),
            2 => c2CircletoCapsule(*(a as *const c2Circle), *(b as *const c2Capsule)),
            _ => 0,
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

    result += c2Collided(
        &circle_in as *const c2Circle as *const u8,
        &circle as *const c2Circle as *const u8,
        0,
    );

    result += c2Collided(
        &circle_in as *const c2Circle as *const u8,
        &aabb as *const c2AABB as *const u8,
        1,
    ) << 1;

    result += c2Collided(
        &circle_in as *const c2Circle as *const u8,
        &capsule as *const c2Capsule as *const u8,
        2,
    ) << 2;

    result
}
