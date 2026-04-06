use std::os::raw::c_int;

#[derive(Clone, Copy)]
#[repr(C)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
enum C2Type {
    Circle = 0,
    AABB = 1,
    Capsule = 2,
}

fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> c_int {
    let n = c2_sub(b.b, b.a);
    let ap = c2_sub(a.p, b.a);
    let da = c2_dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = c2_dot(ap, ap);
    } else {
        let db = c2_dot(c2_sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2_sub(ap, c2_mulvs(n, da / c2_dot(n, n)));
            d2 = c2_dot(e, e);
        } else {
            let bp = c2_sub(a.p, b.b);
            d2 = c2_dot(bp, bp);
        }
    }
    let r = a.r + b.r;
    (d2 < r * r) as c_int
}

fn c2_collided(a: *const u8, b: *const u8, type_b: C2Type) -> c_int {
    unsafe {
        match type_b {
            C2Type::Circle => {
                c2_circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle))
            }
            C2Type::AABB => c2_circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB)),
            C2Type::Capsule => {
                c2_circle_to_capsule(*(a as *const C2Circle), *(b as *const C2Capsule))
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let circle_in = C2Circle {
        p: c2v(x, y),
        r,
    };

    let circle = C2Circle {
        p: c2v(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = C2AABB {
        min: c2v(-40.0, -40.0),
        max: c2v(-15.0, -15.0),
    };

    let capsule = C2Capsule {
        a: c2v(-40.0, 40.0),
        b: c2v(-20.0, 100.0),
        r: 10.0,
    };

    result += c2_collided(
        &circle_in as *const C2Circle as *const u8,
        &circle as *const C2Circle as *const u8,
        C2Type::Circle,
    );

    result += c2_collided(
        &circle_in as *const C2Circle as *const u8,
        &aabb as *const C2AABB as *const u8,
        C2Type::AABB,
    ) << 1;

    result += c2_collided(
        &circle_in as *const C2Circle as *const u8,
        &capsule as *const C2Capsule as *const u8,
        C2Type::Capsule,
    ) << 2;

    result
}
