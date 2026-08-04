// Translation of c_src/src/lib.c

#[derive(Copy, Clone)]
pub enum C2Type {
    Circle = 0,
    Aabb = 1,
    Capsule = 2,
}

#[derive(Copy, Clone)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[derive(Copy, Clone)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[derive(Copy, Clone)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

pub fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

pub fn c2_mulvs(a: C2v, b: f32) -> C2v {
    C2v {
        x: a.x * b,
        y: a.y * b,
    }
}

pub fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

pub fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

pub fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

pub fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

pub fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

pub fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> i32 {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
    if d2 < r2 { 1 } else { 0 }
}

pub fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> i32 {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 { 1 } else { 0 }
}

pub fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> i32 {
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
    if d2 < r * r { 1 } else { 0 }
}

pub enum ShapeRef<'a> {
    Circle(&'a C2Circle),
    Aabb(&'a C2AABB),
    Capsule(&'a C2Capsule),
}

pub fn c2_collided(a: &C2Circle, b: ShapeRef, type_b: C2Type) -> i32 {
    match type_b {
        C2Type::Circle => match b {
            ShapeRef::Circle(c) => c2_circle_to_circle(*a, *c),
            _ => 0,
        },
        C2Type::Aabb => match b {
            ShapeRef::Aabb(c) => c2_circle_to_aabb(*a, *c),
            _ => 0,
        },
        C2Type::Capsule => match b {
            ShapeRef::Capsule(c) => c2_circle_to_capsule(*a, *c),
            _ => 0,
        },
    }
}

pub fn circle_collide(x: f32, y: f32, r: f32) -> i32 {
    let mut result: i32 = 0;

    let circle_in = C2Circle {
        p: c2_v(x, y),
        r,
    };

    let circle = C2Circle {
        p: c2_v(-70.0f32, 0.0),
        r: 20.0f32,
    };

    let aabb = C2AABB {
        min: c2_v(-40.0f32, -40.0f32),
        max: c2_v(-15.0f32, -15.0f32),
    };

    let capsule = C2Capsule {
        a: c2_v(-40.0f32, 40.0f32),
        b: c2_v(-20.0f32, 100.0f32),
        r: 10.0f32,
    };

    result += c2_collided(&circle_in, ShapeRef::Circle(&circle), C2Type::Circle);
    result += c2_collided(&circle_in, ShapeRef::Aabb(&aabb), C2Type::Aabb) << 1;
    result += c2_collided(&circle_in, ShapeRef::Capsule(&capsule), C2Type::Capsule) << 2;

    result
}
