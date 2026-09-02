//! Rust translation of the C library in `c_src/` (a slice of the cute_c2 /
//! tinyc2 2D collision routines plus the `poly_ray` driver entry point).
//!
//! Every public C function is re-exported here with its exact C signature and
//! linker name so the resulting cdylib is ABI-compatible with the original
//! shared library. Behaviour — including the original code's quirks such as
//! `x < 0 ? -x : x` style "abs" (which preserves `-0.0`) and the ternary based
//! min/max (which differ from `f32::min`/`f32::max` on NaN) — is reproduced
//! exactly rather than "fixed".

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Public types (from include/lib.h)
// ---------------------------------------------------------------------------

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Raycast { float t; c2v n; } c2Raycast;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

// ---------------------------------------------------------------------------
// Private types (from src/lib.c)
// ---------------------------------------------------------------------------

// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, C2_TYPE_POLY } C2_TYPE;
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;
const C2_TYPE_POLY: c_int = 3;

/// `typedef struct c2r { float c; float s; } c2r;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

/// `typedef struct c2x { c2v p; c2r r; } c2x;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// `typedef struct c2Capsule { c2v a; c2v b; float r; } c2Capsule;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

/// `typedef struct c2Poly { int count; c2v verts[8]; c2v norms[8]; } c2Poly;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

/// `typedef struct c2Ray { c2v p; c2v d; float t; } c2Ray;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

/// `typedef struct c2m { c2v x; c2v y; } c2m;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

// ---------------------------------------------------------------------------
// Helpers reproducing the C macros used by the original source.
//
// The C code inlines `a < 0 ? -a : a` for absolute value and
// `a < b ? a : b` / `a > b ? a : b` for min/max. These are NOT the same as
// `f32::abs` / `f32::min` / `f32::max` for `-0.0` and NaN inputs, so they are
// spelled out literally here.
// ---------------------------------------------------------------------------

#[inline(always)]
fn c_abs(a: f32) -> f32 {
    if a < 0.0 {
        -a
    } else {
        a
    }
}

#[inline(always)]
fn c_min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
fn c_max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
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
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(c_min(a.x, b.x), c_min(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(c_max(a.x, b.x), c_max(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(c_abs(a.x), c_abs(a.y))
}

// ---------------------------------------------------------------------------
// Ray / shape queries
// ---------------------------------------------------------------------------

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
        (*out).t = t;
        let impact = c2Add(A.p, c2Mulvs(A.d, t));
        (*out).n = c2Norm(c2Sub(impact, p));
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[inline(always)]
fn c2SignedDistPointToPlane_OneDimensional(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

#[inline(always)]
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
    let p0 = A.p;
    let p1 = c2Add(A.p, c2Mulvs(A.d, A.t));
    let mut a_box = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 0.0, y: 0.0 },
    };
    a_box.min = c2Minv(p0, p1);
    a_box.max = c2Maxv(p0, p1);
    if c2AABBtoAABB(a_box, B) == 0 {
        return 0;
    }
    let ab = c2Sub(p1, p0);
    let n = c2Skew(ab);
    let abs_n = c2Absv(n);
    let half_extents = c2Mulvs(c2Sub(B.max, B.min), 0.5f32);
    let center_of_b_box = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
    let d = c_abs(c2Dot(n, c2Sub(p0, center_of_b_box))) - c2Dot(abs_n, half_extents);
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
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = a.y;
    b.y = -a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    let mut c = c2v { x: 0.0, y: 0.0 };
    c.x = a.x.x * b.x + a.x.y * b.y;
    c.y = a.y.x * b.x + a.y.y * b.y;
    c
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    let d0 = (B.x < A.min.x) as c_int;
    let d1 = (B.y < A.min.y) as c_int;
    let d2 = (B.x > A.max.x) as c_int;
    let d3 = (B.y > A.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    let n = c2Sub(A.p, B);
    let d2 = c2Dot(n, n);
    (d2 < A.r * A.r) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: *mut c2Raycast) -> c_int {
    let mut M = c2m {
        x: c2v { x: 0.0, y: 0.0 },
        y: c2v { x: 0.0, y: 0.0 },
    };
    M.y = c2Norm(c2Sub(B.b, B.a));
    M.x = c2CCW90(M.y);
    let cap_n = c2Sub(B.b, B.a);
    let yBb = c2MulmvT(M, cap_n);
    let yAp = c2MulmvT(M, c2Sub(A.p, B.a));
    let yAd = c2MulmvT(M, A.d);
    let yAe = c2Add(yAp, c2Mulvs(yAd, A.t));
    let mut capsule_bb = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 0.0, y: 0.0 },
    };
    capsule_bb.min = c2V(-B.r, 0.0);
    capsule_bb.max = c2V(B.r, yBb.y);
    (*out).n = c2Norm(cap_n);
    (*out).t = 0.0;
    if c2AABBtoPoint(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let mut capsule_a = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        let mut capsule_b = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        capsule_a.p = B.a;
        capsule_a.r = B.r;
        capsule_b.p = B.b;
        capsule_b.r = B.r;
        if c2CircleToPoint(capsule_a, A.p) != 0 {
            return 1;
        } else if c2CircleToPoint(capsule_b, A.p) != 0 {
            return 1;
        }
    }
    if yAe.x * yAp.x < 0.0 || c_min(c_abs(yAe.x), c_abs(yAp.x)) < B.r {
        let mut Ca = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        let mut Cb = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        Ca.p = B.a;
        Ca.r = B.r;
        Cb.p = B.b;
        Cb.r = B.r;
        if c_abs(yAp.x) < B.r {
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
                (*out).n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                (*out).t = t * A.t;
                return 1;
            }
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Rotations / transforms
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    let mut r = c2r { c: 0.0, s: 0.0 };
    r.c = 1.0f32;
    r.s = 0.0;
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 0.0, s: 0.0 },
    };
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoPoly(
    A: c2Ray,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    out: *mut c2Raycast,
) -> c_int {
    let bx = if !bx_ptr.is_null() {
        *bx_ptr
    } else {
        c2xIdentity()
    };
    let p = c2MulxvT(bx, A.p);
    let d = c2MulrvT(bx.r, A.d);
    let mut lo: f32 = 0.0;
    let mut hi: f32 = A.t;
    let mut index: c_int = !0;
    let count = (*B).count;
    let mut i: c_int = 0;
    while i < count {
        let idx = i as usize;
        let norms = (*B).norms.as_ptr();
        let verts = (*B).verts.as_ptr();
        let ni = *norms.add(idx);
        let vi = *verts.add(idx);
        let num = c2Dot(ni, c2Sub(vi, p));
        let den = c2Dot(ni, d);
        if den == 0.0 && num < 0.0 {
            return 0;
        } else {
            if den < 0.0 && num < lo * den {
                lo = num / den;
                index = i;
            } else if den > 0.0 && num < hi * den {
                hi = num / den;
            }
        }
        if hi < lo {
            return 0;
        }
        i += 1;
    }
    if index != !0 {
        (*out).t = lo;
        (*out).n = c2Mulrv(bx.r, *(*B).norms.as_ptr().add(index as usize));
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    bx: *const c2x,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => return c2RaytoCircle(A, *(B as *const c2Circle), out),
        C2_TYPE_AABB => return c2RaytoAABB(A, *(B as *const c2AABB), out),
        C2_TYPE_CAPSULE => return c2RaytoCapsule(A, *(B as *const c2Capsule), out),
        C2_TYPE_POLY => return c2RaytoPoly(A, B as *const c2Poly, bx, out),
        _ => {}
    }
    0
}

// ---------------------------------------------------------------------------
// Driver entry point (declared in include/lib.h)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn poly_ray(cast1: *mut c2Raycast, cast2: *mut c2Raycast) -> c_int {
    let mut hit: c_int = 0;

    // The C code leaves `p` uninitialized and only fills the first four
    // vertices/normals; `count` is 4 so the remaining slots are never read.
    let zero = c2v { x: 0.0, y: 0.0 };
    let mut p = c2Poly {
        count: 0,
        verts: [zero; 8],
        norms: [zero; 8],
    };
    p.verts[0] = c2V(0.875f32, -11.5f32);
    p.verts[1] = c2V(0.875f32, 11.5f32);
    p.verts[2] = c2V(-0.875f32, 11.5f32);
    p.verts[3] = c2V(-0.875f32, -11.5f32);
    p.norms[0] = c2V(1.0, 0.0);
    p.norms[1] = c2V(0.0, 1.0);
    p.norms[2] = c2V(-1.0, 0.0);
    p.norms[3] = c2V(0.0, -1.0);
    p.count = 4;

    let ray0 = c2Ray {
        p: c2v {
            x: -3.869416f32,
            y: 13.0693407f32,
        },
        d: c2v { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    let ray1 = c2Ray {
        p: c2v {
            x: -3.869416f32,
            y: 13.0693407f32,
        },
        d: c2v { x: 0.0, y: -1.0 },
        t: 4.0,
    };

    hit += c2CastRay(
        ray0,
        &p as *const c2Poly as *const c_void,
        std::ptr::null(),
        C2_TYPE_POLY,
        cast1,
    );
    hit += c2CastRay(
        ray1,
        &p as *const c2Poly as *const c_void,
        std::ptr::null(),
        C2_TYPE_POLY,
        cast2,
    ) << 1;

    hit
}
