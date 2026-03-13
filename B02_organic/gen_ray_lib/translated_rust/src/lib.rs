#![allow(non_camel_case_types, non_snake_case, unused_assignments)]

// ---- Structs ----

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Ray {
    p: c2v,
    d: c2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2m {
    x: c2v,
    y: c2v,
}

#[repr(i32)]
#[derive(Clone, Copy)]
enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

// ---- Vector helpers ----

fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

fn c2Add(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x + b.x, y: a.y + b.y }
}

fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}

fn c2Mulvs(a: c2v, b: f32) -> c2v {
    c2v { x: a.x * b, y: a.y * b }
}

fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

// ---- Point tests ----

fn c2AABBtoPoint(a: c2AABB, b: c2v) -> i32 {
    let d0 = (b.x < a.min.x) as i32;
    let d1 = (b.y < a.min.y) as i32;
    let d2 = (b.x > a.max.x) as i32;
    let d3 = (b.y > a.max.y) as i32;
    (!(d0 | d1 | d2 | d3 != 0)) as i32
}

fn c2CircleToPoint(a: c2Circle, b: c2v) -> i32 {
    let n = c2Sub(a.p, b);
    let d2 = c2Dot(n, n);
    (d2 < a.r * a.r) as i32
}

fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    (!(d0 | d1 | d2 | d3 != 0)) as i32
}

// ---- Ray-to-shape ----

fn c2RaytoCircle(a: c2Ray, b: c2Circle, out: &mut c2Raycast) -> i32 {
    let p = b.p;
    let m = c2Sub(a.p, p);
    let c = c2Dot(m, m) - b.r * b.r;
    let bv = c2Dot(m, a.d);
    let disc = bv * bv - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -bv - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        out.t = t;
        let impact = c2Add(a.p, c2Mulvs(a.d, t));
        out.n = c2Norm(c2Sub(impact, p));
        return 1;
    }
    0
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
        if d != 0.0 { da / d } else { 0.0 }
    }
}

fn c2RaytoAABB(a: c2Ray, b: c2AABB, out: &mut c2Raycast) -> i32 {
    let p0 = a.p;
    let p1 = c2Add(a.p, c2Mulvs(a.d, a.t));
    let a_box = c2AABB {
        min: c2Minv(p0, p1),
        max: c2Maxv(p0, p1),
    };
    if c2AABBtoAABB(a_box, b) == 0 {
        return 0;
    }
    let ab = c2Sub(p1, p0);
    let n = c2Skew(ab);
    let abs_n = c2Absv(n);
    let half_extents = c2Mulvs(c2Sub(b.max, b.min), 0.5f32);
    let center_of_b_box = c2Mulvs(c2Add(b.min, b.max), 0.5f32);
    let dot_val = c2Dot(n, c2Sub(p0, center_of_b_box));
    let d = (if dot_val < 0.0 { -dot_val } else { dot_val }) - c2Dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2SignedDistPointToPlane_OneDimensional(p0.x, -1.0, b.min.x);
    let db0 = c2SignedDistPointToPlane_OneDimensional(p1.x, -1.0, b.min.x);
    let da1 = c2SignedDistPointToPlane_OneDimensional(p0.x, 1.0, b.max.x);
    let db1 = c2SignedDistPointToPlane_OneDimensional(p1.x, 1.0, b.max.x);
    let da2 = c2SignedDistPointToPlane_OneDimensional(p0.y, -1.0, b.min.y);
    let db2 = c2SignedDistPointToPlane_OneDimensional(p1.y, -1.0, b.min.y);
    let da3 = c2SignedDistPointToPlane_OneDimensional(p0.y, 1.0, b.max.y);
    let db3 = c2SignedDistPointToPlane_OneDimensional(p1.y, 1.0, b.max.y);
    let t0 = c2RayToPlane_OneDimensional(da0, db0);
    let t1 = c2RayToPlane_OneDimensional(da1, db1);
    let t2 = c2RayToPlane_OneDimensional(da2, db2);
    let t3 = c2RayToPlane_OneDimensional(da3, db3);
    let hit0 = (t0 <= 1.0) as i32;
    let hit1 = (t1 <= 1.0) as i32;
    let hit2 = (t2 <= 1.0) as i32;
    let hit3 = (t3 <= 1.0) as i32;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        let t0 = (hit0 as f32) * t0;
        let t1 = (hit1 as f32) * t1;
        let t2 = (hit2 as f32) * t2;
        let t3 = (hit3 as f32) * t3;
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * a.t;
            out.n = c2V(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * a.t;
            out.n = c2V(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * a.t;
            out.n = c2V(0.0, -1.0);
        } else {
            out.t = t3 * a.t;
            out.n = c2V(0.0, 1.0);
        }
        1
    } else {
        0
    }
}

fn c2RaytoCapsule(a: c2Ray, b: c2Capsule, out: &mut c2Raycast) -> i32 {
    let m = c2m {
        y: c2Norm(c2Sub(b.b, b.a)),
        x: c2CCW90(c2Norm(c2Sub(b.b, b.a))),
    };
    let cap_n = c2Sub(b.b, b.a);
    let yBb = c2MulmvT(m, cap_n);
    let yAp = c2MulmvT(m, c2Sub(a.p, b.a));
    let yAd = c2MulmvT(m, a.d);
    let yAe = c2Add(yAp, c2Mulvs(yAd, a.t));
    let capsule_bb = c2AABB {
        min: c2V(-b.r, 0.0),
        max: c2V(b.r, yBb.y),
    };
    out.n = c2Norm(cap_n);
    out.t = 0.0;
    if c2AABBtoPoint(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let capsule_a = c2Circle { p: b.a, r: b.r };
        let capsule_b = c2Circle { p: b.b, r: b.r };
        if c2CircleToPoint(capsule_a, a.p) != 0 {
            return 1;
        } else if c2CircleToPoint(capsule_b, a.p) != 0 {
            return 1;
        }
    }
    let abs_yAe_x = if yAe.x < 0.0 { -yAe.x } else { yAe.x };
    let abs_yAp_x = if yAp.x < 0.0 { -yAp.x } else { yAp.x };
    let min_abs = if abs_yAe_x < abs_yAp_x { abs_yAe_x } else { abs_yAp_x };
    if yAe.x * yAp.x < 0.0 || min_abs < b.r {
        let ca = c2Circle { p: b.a, r: b.r };
        let cb = c2Circle { p: b.b, r: b.r };
        if abs_yAp_x < b.r {
            if yAp.y < 0.0 {
                return c2RaytoCircle(a, ca, out);
            } else {
                return c2RaytoCircle(a, cb, out);
            }
        } else {
            let c = if yAp.x > 0.0 { b.r } else { -b.r };
            let d = yAe.x - yAp.x;
            let t = (c - yAp.x) / d;
            let y = yAp.y + (yAe.y - yAp.y) * t;
            if y <= 0.0 {
                return c2RaytoCircle(a, ca, out);
            }
            if y >= yBb.y {
                return c2RaytoCircle(a, cb, out);
            } else {
                out.n = if c > 0.0 { m.x } else { c2Skew(m.y) };
                out.t = t * a.t;
                return 1;
            }
        }
    }
    0
}

fn c2CastRay(a: c2Ray, b: *const u8, type_b: C2_TYPE, out: &mut c2Raycast) -> i32 {
    unsafe {
        match type_b {
            C2_TYPE::C2_TYPE_CIRCLE => c2RaytoCircle(a, *(b as *const c2Circle), out),
            C2_TYPE::C2_TYPE_AABB => c2RaytoAABB(a, *(b as *const c2AABB), out),
            C2_TYPE::C2_TYPE_CAPSULE => c2RaytoCapsule(a, *(b as *const c2Capsule), out),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gen_ray(
    cast1: *mut c2Raycast,
    cast2: *mut c2Raycast,
    cast3: *mut c2Raycast,
    mp_x: f32, mp_y: f32,
    r_p_x: f32, r_p_y: f32,
    c_p_x: f32, c_p_y: f32, c_r: f32,
    cap_a_x: f32, cap_a_y: f32, cap_b_x: f32, cap_b_y: f32, cap_r: f32,
    bb_min_x: f32, bb_min_y: f32, bb_max_x: f32, bb_max_y: f32,
) -> std::ffi::c_int {
    unsafe {
        let mut hit: i32 = 0;
        let mp = c2V(mp_x, mp_y);
        let mut ray = c2Ray {
            p: c2V(r_p_x, r_p_y),
            d: c2v { x: 0.0, y: 0.0 },
            t: 0.0,
        };
        ray.d = c2Norm(c2Sub(mp, ray.p));
        ray.t = c2Dot(mp, ray.d) - c2Dot(ray.p, ray.d);

        let c = c2Circle { p: c2V(c_p_x, c_p_y), r: c_r };
        hit += c2CastRay(ray, &c as *const c2Circle as *const u8, C2_TYPE::C2_TYPE_CIRCLE, &mut *cast1);

        let cap = c2Capsule {
            a: c2V(cap_a_x, cap_a_y),
            b: c2V(cap_b_x, cap_b_y),
            r: cap_r,
        };
        hit += c2CastRay(ray, &cap as *const c2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, &mut *cast2) << 1;

        let bb = c2AABB {
            min: c2V(bb_min_x, bb_min_y),
            max: c2V(bb_max_x, bb_max_y),
        };
        hit += c2CastRay(ray, &bb as *const c2AABB as *const u8, C2_TYPE::C2_TYPE_AABB, &mut *cast3) << 2;

        hit
    }
}
