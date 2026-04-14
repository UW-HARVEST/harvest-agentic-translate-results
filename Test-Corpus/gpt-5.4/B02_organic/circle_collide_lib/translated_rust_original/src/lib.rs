use std::os::raw::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
enum C2Type {
    Circle,
    Aabb,
    Capsule,
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
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(a: C2v, b: f32) -> C2v {
    C2v {
        x: a.x * b,
        y: a.y * b,
    }
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
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    (d2 < r2 * r2) as c_int
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> c_int {
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
    let d2 = if da < 0.0 {
        c2_dot(ap, ap)
    } else {
        let db = c2_dot(c2_sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2_sub(ap, c2_mulvs(n, da / c2_dot(n, n)));
            c2_dot(e, e)
        } else {
            let bp = c2_sub(a.p, b.b);
            c2_dot(bp, bp)
        }
    };
    let r = a.r + b.r;
    (d2 < r * r) as c_int
}

fn c2_collided_circle(a: &C2Circle, type_b: C2Type, b_circle: Option<&C2Circle>, b_aabb: Option<&C2Aabb>, b_capsule: Option<&C2Capsule>) -> c_int {
    match type_b {
        C2Type::Circle => c2_circle_to_circle(*a, *b_circle.unwrap()),
        C2Type::Aabb => c2_circle_to_aabb(*a, *b_aabb.unwrap()),
        C2Type::Capsule => c2_circle_to_capsule(*a, *b_capsule.unwrap()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result = 0;

    let circle_in = C2Circle {
        p: c2_v(x, y),
        r,
    };

    let circle = C2Circle {
        p: c2_v(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = C2Aabb {
        min: c2_v(-40.0, -40.0),
        max: c2_v(-15.0, -15.0),
    };

    let capsule = C2Capsule {
        a: c2_v(-40.0, 40.0),
        b: c2_v(-20.0, 100.0),
        r: 10.0,
    };

    result += c2_collided_circle(&circle_in, C2Type::Circle, Some(&circle), None, None);
    result += c2_collided_circle(&circle_in, C2Type::Aabb, None, Some(&aabb), None) << 1;
    result += c2_collided_circle(&circle_in, C2Type::Capsule, None, None, Some(&capsule)) << 2;

    result
}
