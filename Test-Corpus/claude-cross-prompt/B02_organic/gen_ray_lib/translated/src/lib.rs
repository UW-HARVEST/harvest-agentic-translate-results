// Translation of c_src/src/lib.c to Rust.
// Preserves the exact semantics of the original C code, including any bugs.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

#[derive(Copy, Clone, Debug)]
pub enum C2_TYPE {
    Circle,
    Aabb,
    Capsule,
}

pub enum C2Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

#[inline]
pub fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
pub fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
pub fn c2Len(a: c2v) -> f32 {
    // C uses sqrtf
    c2Dot(a, a).sqrt()
}

#[inline]
pub fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline]
pub fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
pub fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline]
pub fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

#[inline]
pub fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[inline]
pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[inline]
pub fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

pub fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: &mut c2Raycast) -> i32 {
    let p = B.p;
    let m = c2Sub(A.p, p);
    let c = c2Dot(m, m) - B.r * B.r;
    let b = c2Dot(m, A.d);
    let disc = b * b - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 && t <= A.t {
        out.t = t;
        let impact = c2Add(A.p, c2Mulvs(A.d, t));
        out.n = c2Norm(c2Sub(impact, p));
        return 1;
    }
    0
}

pub fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> i32 {
    let d0 = (B.max.x < A.min.x) as i32;
    let d1 = (A.max.x < B.min.x) as i32;
    let d2 = (B.max.y < A.min.y) as i32;
    let d3 = (A.max.y < B.min.y) as i32;
    if (d0 | d1 | d2 | d3) != 0 {
        0
    } else {
        1
    }
}

#[inline]
fn c2SignedDistPointToPlane_OneDimensional(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

#[inline]
fn c2RayToPlane_OneDimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0f32
    } else {
        let d = da - db;
        if d != 0.0 {
            da / d
        } else {
            0.0
        }
    }
}

pub fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: &mut c2Raycast) -> i32 {
    let p0 = A.p;
    let p1 = c2Add(A.p, c2Mulvs(A.d, A.t));
    let a_box = c2AABB {
        min: c2Minv(p0, p1),
        max: c2Maxv(p0, p1),
    };
    if c2AABBtoAABB(a_box, B) == 0 {
        return 0;
    }
    let ab = c2Sub(p1, p0);
    let n = c2Skew(ab);
    let abs_n = c2Absv(n);
    let half_extents = c2Mulvs(c2Sub(B.max, B.min), 0.5f32);
    let center_of_b_box = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
    let dot_val = c2Dot(n, c2Sub(p0, center_of_b_box));
    let abs_dot = if dot_val < 0.0 { -dot_val } else { dot_val };
    let d = abs_dot - c2Dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2SignedDistPointToPlane_OneDimensional(p0.x, -1.0f32, B.min.x);
    let db0 = c2SignedDistPointToPlane_OneDimensional(p1.x, -1.0f32, B.min.x);
    let da1 = c2SignedDistPointToPlane_OneDimensional(p0.x, 1.0f32, B.max.x);
    let db1 = c2SignedDistPointToPlane_OneDimensional(p1.x, 1.0f32, B.max.x);
    let da2 = c2SignedDistPointToPlane_OneDimensional(p0.y, -1.0f32, B.min.y);
    let db2 = c2SignedDistPointToPlane_OneDimensional(p1.y, -1.0f32, B.min.y);
    let da3 = c2SignedDistPointToPlane_OneDimensional(p0.y, 1.0f32, B.max.y);
    let db3 = c2SignedDistPointToPlane_OneDimensional(p1.y, 1.0f32, B.max.y);
    let mut t0 = c2RayToPlane_OneDimensional(da0, db0);
    let mut t1 = c2RayToPlane_OneDimensional(da1, db1);
    let mut t2 = c2RayToPlane_OneDimensional(da2, db2);
    let mut t3 = c2RayToPlane_OneDimensional(da3, db3);
    let hit0 = (t0 <= 1.0f32) as i32;
    let hit1 = (t1 <= 1.0f32) as i32;
    let hit2 = (t2 <= 1.0f32) as i32;
    let hit3 = (t3 <= 1.0f32) as i32;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = (hit0 as f32) * t0;
        t1 = (hit1 as f32) * t1;
        t2 = (hit2 as f32) * t2;
        t3 = (hit3 as f32) * t3;
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * A.t;
            out.n = c2V(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * A.t;
            out.n = c2V(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * A.t;
            out.n = c2V(0.0, -1.0);
        } else {
            out.t = t3 * A.t;
            out.n = c2V(0.0, 1.0);
        }
        1
    } else {
        0
    }
}

#[inline]
pub fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[inline]
pub fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

pub fn c2AABBtoPoint(A: c2AABB, B: c2v) -> i32 {
    let d0 = (B.x < A.min.x) as i32;
    let d1 = (B.y < A.min.y) as i32;
    let d2 = (B.x > A.max.x) as i32;
    let d3 = (B.y > A.max.y) as i32;
    if (d0 | d1 | d2 | d3) != 0 {
        0
    } else {
        1
    }
}

pub fn c2CircleToPoint(A: c2Circle, B: c2v) -> i32 {
    let n = c2Sub(A.p, B);
    let d2 = c2Dot(n, n);
    (d2 < A.r * A.r) as i32
}

pub fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: &mut c2Raycast) -> i32 {
    let M_y = c2Norm(c2Sub(B.b, B.a));
    let M_x = c2CCW90(M_y);
    let M = c2m { x: M_x, y: M_y };
    let cap_n = c2Sub(B.b, B.a);
    let yBb = c2MulmvT(M, cap_n);
    let yAp = c2MulmvT(M, c2Sub(A.p, B.a));
    let yAd = c2MulmvT(M, A.d);
    let yAe = c2Add(yAp, c2Mulvs(yAd, A.t));
    let capsule_bb = c2AABB {
        min: c2V(-B.r, 0.0),
        max: c2V(B.r, yBb.y),
    };
    out.n = c2Norm(cap_n);
    out.t = 0.0;
    if c2AABBtoPoint(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let capsule_a = c2Circle { p: B.a, r: B.r };
        let capsule_b = c2Circle { p: B.b, r: B.r };
        if c2CircleToPoint(capsule_a, A.p) != 0 {
            return 1;
        } else if c2CircleToPoint(capsule_b, A.p) != 0 {
            return 1;
        }
    }

    let abs_yAe_x = if yAe.x < 0.0 { -yAe.x } else { yAe.x };
    let abs_yAp_x = if yAp.x < 0.0 { -yAp.x } else { yAp.x };
    let min_abs = if abs_yAe_x < abs_yAp_x { abs_yAe_x } else { abs_yAp_x };
    if yAe.x * yAp.x < 0.0 || min_abs < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        let abs_yAp_x2 = if yAp.x < 0.0 { -yAp.x } else { yAp.x };
        if abs_yAp_x2 < B.r {
            if yAp.y < 0.0 {
                return c2RaytoCircle(A, Ca, out);
            } else {
                return c2RaytoCircle(A, Cb, out);
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = yAe.x - yAp.x;
            let t = (c - yAp.x) / d;
            let y = yAp.y + (yAe.y - yAp.y) * t;
            if y <= 0.0 {
                return c2RaytoCircle(A, Ca, out);
            }
            if y >= yBb.y {
                return c2RaytoCircle(A, Cb, out);
            } else {
                out.n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                out.t = t * A.t;
                return 1;
            }
        }
    }
    0
}

pub fn c2CastRay(A: c2Ray, B: &C2Shape, out: &mut c2Raycast) -> i32 {
    match B {
        C2Shape::Circle(c) => c2RaytoCircle(A, *c, out),
        C2Shape::Aabb(b) => c2RaytoAABB(A, *b, out),
        C2Shape::Capsule(cap) => c2RaytoCapsule(A, *cap, out),
    }
}

pub fn gen_ray(
    cast1: &mut c2Raycast,
    cast2: &mut c2Raycast,
    cast3: &mut c2Raycast,
    mp_x: f32,
    mp_y: f32,
    r_p_x: f32,
    r_p_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    cap_a_x: f32,
    cap_a_y: f32,
    cap_b_x: f32,
    cap_b_y: f32,
    cap_r: f32,
    bb_min_x: f32,
    bb_min_y: f32,
    bb_max_x: f32,
    bb_max_y: f32,
) -> i32 {
    let mut hit: i32 = 0;

    let mp = c2V(mp_x, mp_y);

    let mut ray = c2Ray::default();
    ray.p = c2V(r_p_x, r_p_y);
    ray.d = c2Norm(c2Sub(mp, ray.p));
    ray.t = c2Dot(mp, ray.d) - c2Dot(ray.p, ray.d);

    let c = c2Circle {
        p: c2V(c_p_x, c_p_y),
        r: c_r,
    };

    hit += c2CastRay(ray, &C2Shape::Circle(c), cast1);

    let cap = c2Capsule {
        a: c2V(cap_a_x, cap_a_y),
        b: c2V(cap_b_x, cap_b_y),
        r: cap_r,
    };

    hit += c2CastRay(ray, &C2Shape::Capsule(cap), cast2) << 1;

    let bb = c2AABB {
        min: c2V(bb_min_x, bb_min_y),
        max: c2V(bb_max_x, bb_max_y),
    };

    hit += c2CastRay(ray, &C2Shape::Aabb(bb), cast3) << 2;

    hit
}
