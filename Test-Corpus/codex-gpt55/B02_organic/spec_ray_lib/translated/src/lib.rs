use std::ffi::{c_float, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Raycast {
    pub t: c_float,
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[inline]
fn bool_to_int(v: bool) -> c_int {
    if v { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> c_float {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> c_float {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: c_float) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
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
        unsafe {
            (*out).t = t;
        }
        let impact = c2Add(A.p, c2Mulvs(A.d, t));
        unsafe {
            (*out).n = c2Norm(c2Sub(impact, p));
        }
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = bool_to_int(B.max.x < A.min.x);
    let d1 = bool_to_int(A.max.x < B.min.x);
    let d2 = bool_to_int(B.max.y < A.min.y);
    let d3 = bool_to_int(A.max.y < B.min.y);
    bool_to_int((d0 | d1 | d2 | d3) == 0)
}

#[inline]
fn c2SignedDistPointToPlane_OneDimensional(
    p: c_float,
    n: c_float,
    d: c_float,
) -> c_float {
    p * n - d * n
}

#[inline]
fn c2RayToPlane_OneDimensional(da: c_float, db: c_float) -> c_float {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0
    } else {
        let d = da - db;
        if d != 0.0 { da / d } else { 0.0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
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
    let d = if c2Dot(n, c2Sub(p0, center_of_b_box)) < 0.0 {
        -c2Dot(n, c2Sub(p0, center_of_b_box))
    } else {
        c2Dot(n, c2Sub(p0, center_of_b_box))
    } - c2Dot(abs_n, half_extents);
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
    let mut t0 = c2RayToPlane_OneDimensional(da0, db0);
    let mut t1 = c2RayToPlane_OneDimensional(da1, db1);
    let mut t2 = c2RayToPlane_OneDimensional(da2, db2);
    let mut t3 = c2RayToPlane_OneDimensional(da3, db3);
    let hit0 = bool_to_int(t0 <= 1.0);
    let hit1 = bool_to_int(t1 <= 1.0);
    let hit2 = bool_to_int(t2 <= 1.0);
    let hit3 = bool_to_int(t3 <= 1.0);
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = (hit0 as c_float) * t0;
        t1 = (hit1 as c_float) * t1;
        t2 = (hit2 as c_float) * t2;
        t3 = (hit3 as c_float) * t3;
        unsafe {
            if t0 >= t1 && t0 >= t2 && t0 >= t3 {
                (*out).t = t0 * A.t;
                (*out).n = c2V(-1.0, 0.0);
            } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
                (*out).t = t1 * A.t;
                (*out).n = c2V(1.0, 0.0);
            } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
                (*out).t = t2 * A.t;
                (*out).n = c2V(0.0, -1.0);
            } else {
                (*out).t = t3 * A.t;
                (*out).n = c2V(0.0, 1.0);
            }
        }
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    let d0 = bool_to_int(B.x < A.min.x);
    let d1 = bool_to_int(B.y < A.min.y);
    let d2 = bool_to_int(B.x > A.max.x);
    let d3 = bool_to_int(B.y > A.max.y);
    bool_to_int((d0 | d1 | d2 | d3) == 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    let n = c2Sub(A.p, B);
    let d2 = c2Dot(n, n);
    bool_to_int(d2 < A.r * A.r)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCapsule(
    A: c2Ray,
    B: c2Capsule,
    out: *mut c2Raycast,
) -> c_int {
    let mut M = c2m {
        x: c2V(0.0, 0.0),
        y: c2V(0.0, 0.0),
    };
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
    unsafe {
        (*out).n = c2Norm(cap_n);
        (*out).t = 0.0;
    }
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
    if yAe.x * yAp.x < 0.0
        || (if if yAe.x < 0.0 { -yAe.x } else { yAe.x }
            < if yAp.x < 0.0 { -yAp.x } else { yAp.x }
        {
            if yAe.x < 0.0 { -yAe.x } else { yAe.x }
        } else {
            if yAp.x < 0.0 { -yAp.x } else { yAp.x }
        }) < B.r
    {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if (if yAp.x < 0.0 { -yAp.x } else { yAp.x }) < B.r {
            if yAp.y < 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            } else {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = yAe.x - yAp.x;
            let t = (c - yAp.x) / d;
            let y = yAp.y + (yAe.y - yAp.y) * t;
            if y <= 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            }
            if y >= yBb.y {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                    (*out).t = t * A.t;
                }
                return 1;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => unsafe { c2RaytoCircle(A, *(B as *const c2Circle), out) },
        C2_TYPE_AABB => unsafe { c2RaytoAABB(A, *(B as *const c2AABB), out) },
        C2_TYPE_CAPSULE => unsafe { c2RaytoCapsule(A, *(B as *const c2Capsule), out) },
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spec_ray(
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

    let hit = unsafe { c2CastRay(ray, &c as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, cast) };
    hit
}
