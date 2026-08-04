use std::os::raw::{c_float, c_int};

#[repr(C)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
pub struct c2Raycast {
    pub t: c_float,
    pub n: c2v,
}

#[repr(C)]
enum C2_TYPE {
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
    C2_TYPE_CAPSULE,
}

#[repr(C)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[repr(C)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c_float,
}

#[repr(C)]
struct c2Ray {
    p: c2v,
    d: c2v,
    t: c_float,
}

#[repr(C)]
struct c2m {
    x: c2v,
    y: c2v,
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

fn c2Add(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2Mulvs(a: c2v, b: c_float) -> c2v {
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

fn c2Skew(a: c2v) -> c2v {
    c2v {
        x: -a.y,
        y: a.x,
    }
}

fn c2Absv(a: c2v) -> c2v {
    c2v {
        x: if a.x < 0.0 { -a.x } else { a.x },
        y: if a.y < 0.0 { -a.y } else { a.y },
    }
}

fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: &mut c2Raycast) -> c_int {
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

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = if B.max.x < A.min.x { 1 } else { 0 };
    let d1 = if A.max.x < B.min.x { 1 } else { 0 };
    let d2 = if B.max.y < A.min.y { 1 } else { 0 };
    let d3 = if A.max.y < B.min.y { 1 } else { 0 };
    if (d0 | d1 | d2 | d3) != 0 { 0 } else { 1 }
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

fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: &mut c2Raycast) -> c_int {
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
    let half_extents = c2Mulvs(c2Sub(B.max, B.min), 0.5);
    let center_of_b_box = c2Mulvs(c2Add(B.min, B.max), 0.5);
    let dot_n = c2Dot(n, c2Sub(p0, center_of_b_box));
    let d = if dot_n < 0.0 { -dot_n } else { dot_n } - c2Dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2SignedDistPointToPlane_OneDimensional(p0.x, -1.0, B.min.x);
    let db0 = c2SignedDistPointToPlane_OneDimensional(p1.x, -1.0, B.min.x);
    let da1 = c2SignedDistPointToPlane_OneDimensional(p0.x, 1.0, B.max.x);
    let db1 = c2SignedDistPointToPlane_OneDimensional(p1.x, 1.0, B.max.x);
    let da2 = c2SignedDistPointToPlane_OneDimensional(p0.y, -1.0, B.min.y);
    let db2 = c2SignedDistPointToPlane_OneDimensional(p1.y, -1.0, B.min.y);
    let da3 = c2SignedDistPointToPlane_OneDimensional(p0.y, 1.0, B.max.y);
    let db3 = c2SignedDistPointToPlane_OneDimensional(p1.y, 1.0, B.max.y);
    let t0 = c2RayToPlane_OneDimensional(da0, db0);
    let t1 = c2RayToPlane_OneDimensional(da1, db1);
    let t2 = c2RayToPlane_OneDimensional(da2, db2);
    let t3 = c2RayToPlane_OneDimensional(da3, db3);
    let hit0 = if t0 <= 1.0 { 1 } else { 0 };
    let hit1 = if t1 <= 1.0 { 1 } else { 0 };
    let hit2 = if t2 <= 1.0 { 1 } else { 0 };
    let hit3 = if t3 <= 1.0 { 1 } else { 0 };
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        let t0 = hit0 as c_float * t0;
        let t1 = hit1 as c_float * t1;
        let t2 = hit2 as c_float * t2;
        let t3 = hit3 as c_float * t3;
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

fn c2CCW90(a: c2v) -> c2v {
    c2v {
        x: a.y,
        y: -a.x,
    }
}

fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    let d0 = if B.x < A.min.x { 1 } else { 0 };
    let d1 = if B.y < A.min.y { 1 } else { 0 };
    let d2 = if B.x > A.max.x { 1 } else { 0 };
    let d3 = if B.y > A.max.y { 1 } else { 0 };
    if (d0 | d1 | d2 | d3) != 0 { 0 } else { 1 }
}

fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    let n = c2Sub(A.p, B);
    let d2 = c2Dot(n, n);
    if d2 < A.r * A.r { 1 } else { 0 }
}

fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: &mut c2Raycast) -> c_int {
    let mut M: c2m;
    M.y = c2Norm(c2Sub(B.b, B.a));
    M.x = c2CCW90(M.y);
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
    let yAe_abs = if yAe.x < 0.0 { -yAe.x } else { yAe.x };
    let yAp_abs = if yAp.x < 0.0 { -yAp.x } else { yAp.x };
    let min_abs = if yAe_abs < yAp_abs { yAe_abs } else { yAp_abs };
    if yAe.x * yAp.x < 0.0 || min_abs < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if yAp_abs < B.r {
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

fn c2CastRay(A: c2Ray, B: *const u8, typeB: C2_TYPE, out: &mut c2Raycast) -> c_int {
    unsafe {
        match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => c2RaytoCircle(A, *(B as *const c2Circle), out),
            C2_TYPE::C2_TYPE_AABB => c2RaytoAABB(A, *(B as *const c2AABB), out),
            C2_TYPE::C2_TYPE_CAPSULE => c2RaytoCapsule(A, *(B as *const c2Capsule), out),
        }
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
    let ray = c2Ray {
        p: c2V(r_p_x, r_p_y),
        d: c2Norm(c2Sub(mp, c2V(r_p_x, r_p_y))),
        t: c2Dot(mp, c2Norm(c2Sub(mp, c2V(r_p_x, r_p_y)))) - c2Dot(c2V(r_p_x, r_p_y), c2Norm(c2Sub(mp, c2V(r_p_x, r_p_y)))),
    };
    let d = c2Norm(c2Sub(mp, c2V(r_p_x, r_p_y)));
    let ray = c2Ray {
        p: c2V(r_p_x, r_p_y),
        d: d,
        t: c2Dot(mp, d) - c2Dot(c2V(r_p_x, r_p_y), d),
    };
    unsafe {
        c2CastRay(ray, &c as *const _ as *const u8, C2_TYPE::C2_TYPE_CIRCLE, &mut *cast)
    }
}
