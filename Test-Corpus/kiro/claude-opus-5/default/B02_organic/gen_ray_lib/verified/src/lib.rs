//! Rust translation of `c_src/src/lib.c` (a subset of cute_c2 raycasting).
//!
//! Behaviour is reproduced exactly, including quirks of the original C code.
//!
//! C identifiers are kept verbatim so the translation can be diffed against
//! the original source.
#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum C2_TYPE {
    Circle,
    Aabb,
    Capsule,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

/// The shape argument of `c2CastRay`, standing in for the `const void *B`
/// of the original.
#[derive(Copy, Clone)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

#[inline]
fn c2V_impl(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
fn c2Dot_impl(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
fn c2Len_impl(a: c2v) -> f32 {
    c2Dot_impl(a, a).sqrt()
}

#[inline]
fn c2Add_impl(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline]
fn c2Sub_impl(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
fn c2Mulvs_impl(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline]
fn c2Div_impl(a: c2v, b: f32) -> c2v {
    c2Mulvs_impl(a, 1.0f32 / b)
}

#[inline]
fn c2Norm_impl(a: c2v) -> c2v {
    c2Div_impl(a, c2Len_impl(a))
}

#[inline]
fn c2Minv_impl(a: c2v, b: c2v) -> c2v {
    c2V_impl(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Maxv_impl(a: c2v, b: c2v) -> c2v {
    c2V_impl(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Skew_impl(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[inline]
fn c2Absv_impl(a: c2v) -> c2v {
    c2V_impl(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

#[inline]
fn c2CCW90_impl(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[inline]
fn c2MulmvT_impl(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

fn c2RaytoCircle_impl(A: c2Ray, B: c2Circle, out: &mut c2Raycast) -> c_int {
    let p = B.p;
    let m = c2Sub_impl(A.p, p);
    let c = c2Dot_impl(m, m) - B.r * B.r;
    let b = c2Dot_impl(m, A.d);
    let disc = b * b - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 && t <= A.t {
        out.t = t;
        let impact = c2Add_impl(A.p, c2Mulvs_impl(A.d, t));
        out.n = c2Norm_impl(c2Sub_impl(impact, p));
        return 1;
    }
    0
}

fn c2AABBtoAABB_impl(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
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

fn c2RaytoAABB_impl(A: c2Ray, B: c2AABB, out: &mut c2Raycast) -> c_int {
    let p0 = A.p;
    let p1 = c2Add_impl(A.p, c2Mulvs_impl(A.d, A.t));
    let a_box = c2AABB {
        min: c2Minv_impl(p0, p1),
        max: c2Maxv_impl(p0, p1),
    };
    if c2AABBtoAABB_impl(a_box, B) == 0 {
        return 0;
    }
    let ab = c2Sub_impl(p1, p0);
    let n = c2Skew_impl(ab);
    let abs_n = c2Absv_impl(n);
    let half_extents = c2Mulvs_impl(c2Sub_impl(B.max, B.min), 0.5f32);
    let center_of_b_box = c2Mulvs_impl(c2Add_impl(B.min, B.max), 0.5f32);
    let dot_n = c2Dot_impl(n, c2Sub_impl(p0, center_of_b_box));
    let d = (if dot_n < 0.0 { -dot_n } else { dot_n }) - c2Dot_impl(abs_n, half_extents);
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
    let hit0 = (t0 <= 1.0f32) as c_int;
    let hit1 = (t1 <= 1.0f32) as c_int;
    let hit2 = (t2 <= 1.0f32) as c_int;
    let hit3 = (t3 <= 1.0f32) as c_int;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = (hit0 as f32) * t0;
        t1 = (hit1 as f32) * t1;
        t2 = (hit2 as f32) * t2;
        t3 = (hit3 as f32) * t3;
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * A.t;
            out.n = c2V_impl(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * A.t;
            out.n = c2V_impl(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * A.t;
            out.n = c2V_impl(0.0, -1.0);
        } else {
            out.t = t3 * A.t;
            out.n = c2V_impl(0.0, 1.0);
        }
        1
    } else {
        0
    }
}

fn c2AABBtoPoint_impl(A: c2AABB, B: c2v) -> c_int {
    let d0 = (B.x < A.min.x) as c_int;
    let d1 = (B.y < A.min.y) as c_int;
    let d2 = (B.x > A.max.x) as c_int;
    let d3 = (B.y > A.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn c2CircleToPoint_impl(A: c2Circle, B: c2v) -> c_int {
    let n = c2Sub_impl(A.p, B);
    let d2 = c2Dot_impl(n, n);
    (d2 < A.r * A.r) as c_int
}

fn c2RaytoCapsule_impl(A: c2Ray, B: c2Capsule, out: &mut c2Raycast) -> c_int {
    let mut M = c2m {
        x: c2V_impl(0.0, 0.0),
        y: c2V_impl(0.0, 0.0),
    };
    M.y = c2Norm_impl(c2Sub_impl(B.b, B.a));
    M.x = c2CCW90_impl(M.y);
    let cap_n = c2Sub_impl(B.b, B.a);
    let yBb = c2MulmvT_impl(M, cap_n);
    let yAp = c2MulmvT_impl(M, c2Sub_impl(A.p, B.a));
    let yAd = c2MulmvT_impl(M, A.d);
    let yAe = c2Add_impl(yAp, c2Mulvs_impl(yAd, A.t));
    let capsule_bb = c2AABB {
        min: c2V_impl(-B.r, 0.0),
        max: c2V_impl(B.r, yBb.y),
    };
    out.n = c2Norm_impl(cap_n);
    out.t = 0.0;
    if c2AABBtoPoint_impl(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let capsule_a = c2Circle { p: B.a, r: B.r };
        let capsule_b = c2Circle { p: B.b, r: B.r };
        if c2CircleToPoint_impl(capsule_a, A.p) != 0 {
            return 1;
        } else if c2CircleToPoint_impl(capsule_b, A.p) != 0 {
            return 1;
        }
    }

    let abs_yAe_x = if yAe.x < 0.0 { -yAe.x } else { yAe.x };
    let abs_yAp_x = if yAp.x < 0.0 { -yAp.x } else { yAp.x };
    let min_abs = if abs_yAe_x < abs_yAp_x {
        abs_yAe_x
    } else {
        abs_yAp_x
    };
    if yAe.x * yAp.x < 0.0 || min_abs < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if abs_yAp_x < B.r {
            if yAp.y < 0.0 {
                return c2RaytoCircle_impl(A, Ca, out);
            } else {
                return c2RaytoCircle_impl(A, Cb, out);
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = yAe.x - yAp.x;
            let t = (c - yAp.x) / d;
            let y = yAp.y + (yAe.y - yAp.y) * t;
            if y <= 0.0 {
                return c2RaytoCircle_impl(A, Ca, out);
            }
            if y >= yBb.y {
                return c2RaytoCircle_impl(A, Cb, out);
            } else {
                out.n = if c > 0.0 { M.x } else { c2Skew_impl(M.y) };
                out.t = t * A.t;
                return 1;
            }
        }
    }
    0
}

fn c2CastRay_impl(A: c2Ray, B: Shape, typeB: C2_TYPE, out: &mut c2Raycast) -> c_int {
    match (typeB, B) {
        (C2_TYPE::Circle, Shape::Circle(circle)) => c2RaytoCircle_impl(A, circle, out),
        (C2_TYPE::Aabb, Shape::Aabb(aabb)) => c2RaytoAABB_impl(A, aabb, out),
        (C2_TYPE::Capsule, Shape::Capsule(capsule)) => c2RaytoCapsule_impl(A, capsule, out),
        // Unreachable: the original C falls off the end of the switch here,
        // which is undefined behaviour. Never exercised by `gen_ray`.
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gen_ray(
    cast1: *mut c2Raycast,
    cast2: *mut c2Raycast,
    cast3: *mut c2Raycast,
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
) -> c_int {
    let out1: &mut c2Raycast = unsafe { &mut *cast1 };
    let out2: &mut c2Raycast = unsafe { &mut *cast2 };
    let out3: &mut c2Raycast = unsafe { &mut *cast3 };

    let mut hit: c_int = 0;

    let mp = c2V_impl(mp_x, mp_y);

    let mut ray = c2Ray {
        p: c2V_impl(0.0, 0.0),
        d: c2V_impl(0.0, 0.0),
        t: 0.0,
    };
    ray.p = c2V_impl(r_p_x, r_p_y);
    ray.d = c2Norm_impl(c2Sub_impl(mp, ray.p));
    ray.t = c2Dot_impl(mp, ray.d) - c2Dot_impl(ray.p, ray.d);

    let c = c2Circle {
        p: c2V_impl(c_p_x, c_p_y),
        r: c_r,
    };

    hit += c2CastRay_impl(ray, Shape::Circle(c), C2_TYPE::Circle, out1);

    let cap = c2Capsule {
        a: c2V_impl(cap_a_x, cap_a_y),
        b: c2V_impl(cap_b_x, cap_b_y),
        r: cap_r,
    };

    hit += c2CastRay_impl(ray, Shape::Capsule(cap), C2_TYPE::Capsule, out2) << 1;

    let bb = c2AABB {
        min: c2V_impl(bb_min_x, bb_min_y),
        max: c2V_impl(bb_max_x, bb_max_y),
    };

    hit += c2CastRay_impl(ray, Shape::Aabb(bb), C2_TYPE::Aabb, out3) << 2;

    hit
}

// ---------------------------------------------------------------------------
// C ABI exports.
//
// The original C translation unit compiles every non-`static` function with
// external linkage, so the shared object exports all of them.  The wrappers
// below reproduce that symbol table exactly.  Only
// `c2SignedDistPointToPlane_OneDimensional` and
// `c2RayToPlane_OneDimensional` stay internal, matching their
// `static inline` definitions in the C source.
// ---------------------------------------------------------------------------

use std::ffi::c_void;

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    self::c2V_impl(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    self::c2Dot_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    self::c2Len_impl(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    self::c2Add_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    self::c2Sub_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    self::c2Mulvs_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    self::c2Div_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    self::c2Norm_impl(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    self::c2Minv_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    self::c2Maxv_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    self::c2Skew_impl(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    self::c2Absv_impl(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    self::c2CCW90_impl(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    self::c2MulmvT_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    self::c2AABBtoAABB_impl(A, B)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    self::c2AABBtoPoint_impl(A, B)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    self::c2CircleToPoint_impl(A, B)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    self::c2RaytoCircle_impl(A, B, unsafe { &mut *out })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
    self::c2RaytoAABB_impl(A, B, unsafe { &mut *out })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: *mut c2Raycast) -> c_int {
    self::c2RaytoCapsule_impl(A, B, unsafe { &mut *out })
}

/// `int c2CastRay(c2Ray A, const void *B, C2_TYPE typeB, c2Raycast *out)`
///
/// `typeB` arrives as a plain `int` because that is the promoted type of the
/// C enumeration.  The C `switch` has no `default` label, so any value
/// outside `0..=2` falls off the end of the function; that path is undefined
/// behaviour in C and is reported as `0` here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    let out = unsafe { &mut *out };
    match typeB {
        0 => self::c2RaytoCircle_impl(A, unsafe { *(B as *const c2Circle) }, out),
        1 => self::c2RaytoAABB_impl(A, unsafe { *(B as *const c2AABB) }, out),
        2 => self::c2RaytoCapsule_impl(A, unsafe { *(B as *const c2Capsule) }, out),
        _ => 0,
    }
}
