use std::os::raw::{c_float, c_int};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Raycast {
    pub t: c_float,
    pub n: c2v,
}

#[derive(Copy, Clone, Debug)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[derive(Copy, Clone, Debug)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[derive(Copy, Clone, Debug)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c_float,
}

#[derive(Copy, Clone, Debug)]
struct c2Ray {
    p: c2v,
    d: c2v,
    t: c_float,
}

fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

fn c2Dot(a: c2v, b: c2v) -> c_float {
    a.x * b.x + a.y * b.y
}

fn c2Len(a: c2v) -> c_float {
    c2Dot(a, a).sqrt()
}

fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2Mulvs(mut a: c2v, b: c_float) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
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

fn c2RaytoCircle(a: c2Ray, b: c2Circle, out: &mut c2Raycast) -> c_int {
    let p = b.p;
    let m = c2Sub(a.p, p);
    let c = c2Dot(m, m) - b.r * b.r;
    let b_val = c2Dot(m, a.d);
    let disc = b_val * b_val - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -b_val - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        out.t = t;
        let impact = c2Add(a.p, c2Mulvs(a.d, t));
        out.n = c2Norm(c2Sub(impact, p));
        return 1;
    }
    0
}

fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    if d0 || d1 || d2 || d3 { 0 } else { 1 }
}

fn c2SignedDistPointToPlane_OneDimensional(p: c_float, n: c_float, d: c_float) -> c_float {
    p * n - d * n
}

fn c2RayToPlane_OneDimensional(da: c_float, db: c_float) -> c_float {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0
    } else {
        let d = da - db;
        if d != 0.0 {
            da / d
        } else {
            0.0
        }
    }
}

fn c2RaytoAABB(a: c2Ray, b: c2AABB, out: &mut c2Raycast) -> c_int {
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
    let half_extents = c2Mulvs(c2Sub(b.max, b.min), 0.5);
    let center_of_b_box = c2Mulvs(c2Add(b.min, b.max), 0.5);

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

    let mut t0 = c2RayToPlane_OneDimensional(da0, db0);
    let mut t1 = c2RayToPlane_OneDimensional(da1, db1);
    let mut t2 = c2RayToPlane_OneDimensional(da2, db2);
    let mut t3 = c2RayToPlane_OneDimensional(da3, db3);

    let hit0 = t0 <= 1.0;
    let hit1 = t1 <= 1.0;
    let hit2 = t2 <= 1.0;
    let hit3 = t3 <= 1.0;
    let hit = hit0 || hit1 || hit2 || hit3;

    if hit {
        t0 = if hit0 { t0 } else { 0.0 };
        t1 = if hit1 { t1 } else { 0.0 };
        t2 = if hit2 { t2 } else { 0.0 };
        t3 = if hit3 { t3 } else { 0.0 };

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

#[derive(Copy, Clone, Debug)]
struct c2m {
    x: c2v,
    y: c2v,
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

fn c2AABBtoPoint(a: c2AABB, b: c2v) -> c_int {
    let d0 = b.x < a.min.x;
    let d1 = b.y < a.min.y;
    let d2 = b.x > a.max.x;
    let d3 = b.y > a.max.y;
    if d0 || d1 || d2 || d3 { 0 } else { 1 }
}

fn c2CircleToPoint(a: c2Circle, b: c2v) -> c_int {
    let n = c2Sub(a.p, b);
    let d2 = c2Dot(n, n);
    if d2 < a.r * a.r { 1 } else { 0 }
}

fn c2RaytoCapsule(a: c2Ray, b: c2Capsule, out: &mut c2Raycast) -> c_int {
    let mut m = c2m {
        x: c2V(0.0, 0.0),
        y: c2Norm(c2Sub(b.b, b.a)),
    };
    m.x = c2CCW90(m.y);
    let cap_n = c2Sub(b.b, b.a);
    let y_bb = c2MulmvT(m, cap_n);
    let y_ap = c2MulmvT(m, c2Sub(a.p, b.a));
    let y_ad = c2MulmvT(m, a.d);
    let y_ae = c2Add(y_ap, c2Mulvs(y_ad, a.t));
    let capsule_bb = c2AABB {
        min: c2V(-b.r, 0.0),
        max: c2V(b.r, y_bb.y),
    };
    out.n = c2Norm(cap_n);
    out.t = 0.0;
    if c2AABBtoPoint(capsule_bb, y_ap) != 0 {
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

    let abs_y_ae_x = if y_ae.x < 0.0 { -y_ae.x } else { y_ae.x };
    let abs_y_ap_x = if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x };
    let min_abs = if abs_y_ae_x < abs_y_ap_x { abs_y_ae_x } else { abs_y_ap_x };

    if y_ae.x * y_ap.x < 0.0 || min_abs < b.r {
        let ca = c2Circle { p: b.a, r: b.r };
        let cb = c2Circle { p: b.b, r: b.r };
        if abs_y_ap_x < b.r {
            if y_ap.y < 0.0 {
                return c2RaytoCircle(a, ca, out);
            } else {
                return c2RaytoCircle(a, cb, out);
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return c2RaytoCircle(a, ca, out);
            }
            if y >= y_bb.y {
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

enum C2Shape<'a> {
    Circle(&'a c2Circle),
    AABB(&'a c2AABB),
    Capsule(&'a c2Capsule),
}

fn c2CastRay(a: c2Ray, shape: C2Shape, out: &mut c2Raycast) -> c_int {
    match shape {
        C2Shape::Circle(c) => c2RaytoCircle(a, *c, out),
        C2Shape::AABB(aabb) => c2RaytoAABB(a, *aabb, out),
        C2Shape::Capsule(cap) => c2RaytoCapsule(a, *cap, out),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn spec_ray(
    cast: *mut c2Raycast,
    mp_x: c_float,
    mp_y: c_float,
    c_p_x: c_float,
    c_p_y: c_float,
    c_r: c_float,
    r_p_x: c_float,
    r_p_y: c_float,
) -> c_int {
    let mp = c2V(mp_x, mp_y);
    let c = c2Circle {
        p: c2V(c_p_x, c_p_y),
        r: c_r,
    };
    let mut ray = c2Ray {
        p: c2V(r_p_x, r_p_y),
        d: c2V(0.0, 0.0),
        t: 0.0,
    };
    ray.d = c2Norm(c2Sub(mp, ray.p));
    ray.t = c2Dot(mp, ray.d) - c2Dot(ray.p, ray.d);

    if let Some(out) = unsafe { cast.as_mut() } {
        c2CastRay(ray, C2Shape::Circle(&c), out)
    } else {
        0
    }
}
