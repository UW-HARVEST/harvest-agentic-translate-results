use std::os::raw::{c_float, c_int};

#[repr(C)]
#[derive(Clone, Copy)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
enum C2Type {
    Circle = 0,
    Aabb = 1,
    Capsule = 2,
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
    C2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    C2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
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

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> bool {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
    d2 < r2
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> bool {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    d2 < r2
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> bool {
    let n = c2_sub(b.b, b.a);
    let ap = c2_sub(a.p, b.a);
    let da = c2_dot(ap, n);
    let d2: f32;
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
    d2 < r * r
}

fn c2_collided(a: &C2Circle, b: *const u8, type_b: C2Type) -> bool {
    unsafe {
        match type_b {
            C2Type::Circle => {
                let b = *(b as *const C2Circle);
                c2_circle_to_circle(*a, b)
            }
            C2Type::Aabb => {
                let b = *(b as *const C2Aabb);
                c2_circle_to_aabb(*a, b)
            }
            C2Type::Capsule => {
                let b = *(b as *const C2Capsule);
                c2_circle_to_capsule(*a, b)
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: c_float, y: c_float, r: c_float) -> c_int {
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

    if c2_collided(&circle_in, &circle as *const _ as *const u8, C2Type::Circle) {
        result += 1;
    }

    if c2_collided(&circle_in, &aabb as *const _ as *const u8, C2Type::Aabb) {
        result += 2;
    }

    if c2_collided(&circle_in, &capsule as *const _ as *const u8, C2Type::Capsule) {
        result += 4;
    }

    result
}
