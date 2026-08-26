//! Rust translation of the C library in `c_src/`.
//!
//! This is a faithful, behaviour-preserving translation of `c_src/src/lib.c`
//! (a subset of the `cute_c2` 2D collision routines) plus the public entry
//! point `poly_ray` declared in `c_src/include/lib.h`.
//!
//! Every function that the C shared library exports is re-exported here with
//! the identical linker symbol name, the identical C ABI signature and the
//! identical arithmetic (including the original code's quirks / bugs).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use core::ffi::c_int;

// ---------------------------------------------------------------------------
// Public types (from c_src/include/lib.h)
// ---------------------------------------------------------------------------

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Raycast { float t; c2v n; } c2Raycast;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

// ---------------------------------------------------------------------------
// Private types (from c_src/src/lib.c)
// ---------------------------------------------------------------------------

// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, C2_TYPE_POLY } C2_TYPE;
pub const C2_TYPE_CIRCLE: u32 = 0;
pub const C2_TYPE_AABB: u32 = 1;
pub const C2_TYPE_CAPSULE: u32 = 2;
pub const C2_TYPE_POLY: u32 = 3;

/// `typedef struct c2r { float c; float s; } c2r;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

/// `typedef struct c2x { c2v p; c2r r; } c2x;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// `typedef struct c2Capsule { c2v a; c2v b; float r; } c2Capsule;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

/// `typedef struct c2Poly { int count; c2v verts[8]; c2v norms[8]; } c2Poly;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

impl Default for c2Poly {
    fn default() -> Self {
        c2Poly {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
}

/// `typedef struct c2Ray { c2v p; c2v d; float t; } c2Ray;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

/// `typedef struct c2m { c2v x; c2v y; } c2m;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

// ---------------------------------------------------------------------------
// Internal helpers reproducing the C ternary macros bit-for-bit.
//
// The C source expands `c2Abs`/`c2Min`/`c2Max` style macros into conditional
// expressions.  Those have subtly different semantics from `f32::abs`,
// `f32::min` and `f32::max` for NaN and signed zero, so replicate the
// conditional form exactly instead of using the library intrinsics.
// ---------------------------------------------------------------------------

#[inline(always)]
fn c_abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

#[inline(always)]
fn c_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

#[inline(always)]
fn c_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

// ---------------------------------------------------------------------------
// Vector math
// ---------------------------------------------------------------------------

#[inline]
fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
fn len(a: c2v) -> f32 {
    dot(a, a).sqrt()
}

#[inline]
fn add(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

#[inline]
fn sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[inline]
fn mulvs(a: c2v, b: f32) -> c2v {
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

#[inline]
fn div(a: c2v, b: f32) -> c2v {
    mulvs(a, 1.0f32 / b)
}

#[inline]
fn norm(a: c2v) -> c2v {
    div(a, len(a))
}

#[inline]
fn minv(a: c2v, b: c2v) -> c2v {
    v(c_min(a.x, b.x), c_min(a.y, b.y))
}

#[inline]
fn maxv(a: c2v, b: c2v) -> c2v {
    v(c_max(a.x, b.x), c_max(a.y, b.y))
}

#[inline]
fn skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[inline]
fn absv(a: c2v) -> c2v {
    v(c_abs(a.x), c_abs(a.y))
}

#[inline]
fn ccw90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[inline]
fn mulmv_t(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

#[inline]
fn rot_identity() -> c2r {
    c2r { c: 1.0f32, s: 0.0 }
}

#[inline]
fn x_identity() -> c2x {
    c2x {
        p: v(0.0, 0.0),
        r: rot_identity(),
    }
}

#[inline]
fn mulrv(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
fn mulrv_t(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[inline]
fn mulxv_t(a: c2x, b: c2v) -> c2v {
    mulrv_t(a.r, sub(b, a.p))
}

// ---------------------------------------------------------------------------
// Exported vector math API
// ---------------------------------------------------------------------------

/// `c2v c2V(float x, float y)`
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

/// `float c2Dot(c2v a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

/// `float c2Len(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    dot(a, a).sqrt()
}

/// `c2v c2Add(c2v a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

/// `c2v c2Sub(c2v a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

/// `c2v c2Mulvs(c2v a, float b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

/// `c2v c2Div(c2v a, float b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    mulvs(a, 1.0f32 / b)
}

/// `c2v c2Norm(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    div(a, len(a))
}

/// `c2v c2Minv(c2v a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    v(c_min(a.x, b.x), c_min(a.y, b.y))
}

/// `c2v c2Maxv(c2v a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    v(c_max(a.x, b.x), c_max(a.y, b.y))
}

/// `c2v c2Skew(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

/// `c2v c2Absv(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    v(c_abs(a.x), c_abs(a.y))
}

/// `c2v c2CCW90(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

/// `c2v c2MulmvT(c2m a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

/// `c2r c2RotIdentity(void)`
#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2r { c: 1.0f32, s: 0.0 }
}

/// `c2x c2xIdentity(void)`
#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: v(0.0, 0.0),
        r: rot_identity(),
    }
}

/// `c2v c2Mulrv(c2r a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

/// `c2v c2MulrvT(c2r a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

/// `c2v c2MulxvT(c2x a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    mulrv_t(a.r, sub(b, a.p))
}

// ---------------------------------------------------------------------------
// Overlap tests
// ---------------------------------------------------------------------------

#[inline]
fn aabb_to_aabb(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// `int c2AABBtoAABB(c2AABB A, c2AABB B)`
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    aabb_to_aabb(A, B)
}

#[inline]
fn aabb_to_point(a: c2AABB, b: c2v) -> c_int {
    let d0 = (b.x < a.min.x) as c_int;
    let d1 = (b.y < a.min.y) as c_int;
    let d2 = (b.x > a.max.x) as c_int;
    let d3 = (b.y > a.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// `int c2AABBtoPoint(c2AABB A, c2v B)`
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    aabb_to_point(A, B)
}

#[inline]
fn circle_to_point(a: c2Circle, b: c2v) -> c_int {
    let n = sub(a.p, b);
    let d2 = dot(n, n);
    (d2 < a.r * a.r) as c_int
}

/// `int c2CircleToPoint(c2Circle A, c2v B)`
#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    circle_to_point(A, B)
}

// ---------------------------------------------------------------------------
// Raycasts
// ---------------------------------------------------------------------------

/// `static inline float c2SignedDistPointToPlane_OneDimensional(float p, float n, float d)`
#[inline(always)]
fn signed_dist_point_to_plane_one_dimensional(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

/// `static inline float c2RayToPlane_OneDimensional(float da, float db)`
#[inline(always)]
fn ray_to_plane_one_dimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0f32
    } else {
        let d = da - db;
        if d != 0.0 { da / d } else { 0.0 }
    }
}

/// Shared implementation of `c2RaytoCircle`.
///
/// # Safety
/// `out` must be a valid, writable pointer to a `c2Raycast` whenever the
/// function reaches a branch that stores through it (the C code dereferences
/// it unconditionally on a hit).
unsafe fn rayto_circle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    let p = B.p;
    let m = sub(A.p, p);
    let c = dot(m, m) - B.r * B.r;
    let b = dot(m, A.d);
    let disc = b * b - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 && t <= A.t {
        unsafe {
            (*out).t = t;
        }
        let impact = add(A.p, mulvs(A.d, t));
        unsafe {
            (*out).n = norm(sub(impact, p));
        }
        return 1;
    }
    0
}

/// `int c2RaytoCircle(c2Ray A, c2Circle B, c2Raycast *out)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    unsafe { rayto_circle(A, B, out) }
}

/// Shared implementation of `c2RaytoAABB`.
///
/// # Safety
/// See [`rayto_circle`].
unsafe fn rayto_aabb(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
    let p0 = A.p;
    let p1 = add(A.p, mulvs(A.d, A.t));
    let a_box = c2AABB {
        min: minv(p0, p1),
        max: maxv(p0, p1),
    };
    if aabb_to_aabb(a_box, B) == 0 {
        return 0;
    }
    let ab = sub(p1, p0);
    let n = skew(ab);
    let abs_n = absv(n);
    let half_extents = mulvs(sub(B.max, B.min), 0.5f32);
    let center_of_b_box = mulvs(add(B.min, B.max), 0.5f32);
    let d = c_abs(dot(n, sub(p0, center_of_b_box))) - dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = signed_dist_point_to_plane_one_dimensional(p0.x, -1.0f32, B.min.x);
    let db0 = signed_dist_point_to_plane_one_dimensional(p1.x, -1.0f32, B.min.x);
    let da1 = signed_dist_point_to_plane_one_dimensional(p0.x, 1.0f32, B.max.x);
    let db1 = signed_dist_point_to_plane_one_dimensional(p1.x, 1.0f32, B.max.x);
    let da2 = signed_dist_point_to_plane_one_dimensional(p0.y, -1.0f32, B.min.y);
    let db2 = signed_dist_point_to_plane_one_dimensional(p1.y, -1.0f32, B.min.y);
    let da3 = signed_dist_point_to_plane_one_dimensional(p0.y, 1.0f32, B.max.y);
    let db3 = signed_dist_point_to_plane_one_dimensional(p1.y, 1.0f32, B.max.y);
    let mut t0 = ray_to_plane_one_dimensional(da0, db0);
    let mut t1 = ray_to_plane_one_dimensional(da1, db1);
    let mut t2 = ray_to_plane_one_dimensional(da2, db2);
    let mut t3 = ray_to_plane_one_dimensional(da3, db3);
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
            unsafe {
                (*out).t = t0 * A.t;
                (*out).n = v(-1.0, 0.0);
            }
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            unsafe {
                (*out).t = t1 * A.t;
                (*out).n = v(1.0, 0.0);
            }
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            unsafe {
                (*out).t = t2 * A.t;
                (*out).n = v(0.0, -1.0);
            }
        } else {
            unsafe {
                (*out).t = t3 * A.t;
                (*out).n = v(0.0, 1.0);
            }
        }
        1
    } else {
        0
    }
}

/// `int c2RaytoAABB(c2Ray A, c2AABB B, c2Raycast *out)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
    unsafe { rayto_aabb(A, B, out) }
}

/// Shared implementation of `c2RaytoCapsule`.
///
/// # Safety
/// See [`rayto_circle`].
unsafe fn rayto_capsule(A: c2Ray, B: c2Capsule, out: *mut c2Raycast) -> c_int {
    let mut M = c2m::default();
    M.y = norm(sub(B.b, B.a));
    M.x = ccw90(M.y);
    let cap_n = sub(B.b, B.a);
    let yBb = mulmv_t(M, cap_n);
    let yAp = mulmv_t(M, sub(A.p, B.a));
    let yAd = mulmv_t(M, A.d);
    let yAe = add(yAp, mulvs(yAd, A.t));
    let capsule_bb = c2AABB {
        min: v(-B.r, 0.0),
        max: v(B.r, yBb.y),
    };
    unsafe {
        (*out).n = norm(cap_n);
        (*out).t = 0.0;
    }
    if aabb_to_point(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let capsule_a = c2Circle { p: B.a, r: B.r };
        let capsule_b = c2Circle { p: B.b, r: B.r };
        if circle_to_point(capsule_a, A.p) != 0 {
            return 1;
        } else if circle_to_point(capsule_b, A.p) != 0 {
            return 1;
        }
    }
    if yAe.x * yAp.x < 0.0 || c_min(c_abs(yAe.x), c_abs(yAp.x)) < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if c_abs(yAp.x) < B.r {
            if yAp.y < 0.0 {
                return unsafe { rayto_circle(A, Ca, out) };
            } else {
                return unsafe { rayto_circle(A, Cb, out) };
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = yAe.x - yAp.x;
            let t = (c - yAp.x) / d;
            let y = yAp.y + (yAe.y - yAp.y) * t;
            if y <= 0.0 {
                return unsafe { rayto_circle(A, Ca, out) };
            }
            if y >= yBb.y {
                return unsafe { rayto_circle(A, Cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { M.x } else { skew(M.y) };
                    (*out).t = t * A.t;
                }
                return 1;
            }
        }
    }
    0
}

/// `int c2RaytoCapsule(c2Ray A, c2Capsule B, c2Raycast *out)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: *mut c2Raycast) -> c_int {
    unsafe { rayto_capsule(A, B, out) }
}

/// Shared implementation of `c2RaytoPoly`.
///
/// # Safety
/// `B` must point to a valid `c2Poly` (the C code dereferences it
/// unconditionally).  `bx_ptr` may be null.  See [`rayto_circle`] for `out`.
unsafe fn rayto_poly(
    A: c2Ray,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    out: *mut c2Raycast,
) -> c_int {
    let bx = if !bx_ptr.is_null() {
        unsafe { *bx_ptr }
    } else {
        x_identity()
    };
    let p = mulxv_t(bx, A.p);
    let d = mulrv_t(bx.r, A.d);
    let mut lo: f32 = 0.0;
    let mut hi: f32 = A.t;
    // `int index = ~0;` -> -1
    let mut index: c_int = !0;

    let count = unsafe { (*B).count };
    // Reproduce C's unchecked array indexing (`B->verts[i]` / `B->norms[i]`).
    let verts = unsafe { (&raw const (*B).verts) as *const c2v };
    let norms = unsafe { (&raw const (*B).norms) as *const c2v };

    let mut i: c_int = 0;
    while i < count {
        let ni = unsafe { *norms.offset(i as isize) };
        let vi = unsafe { *verts.offset(i as isize) };
        let num = dot(ni, sub(vi, p));
        let den = dot(ni, d);
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
        unsafe {
            (*out).t = lo;
            (*out).n = mulrv(bx.r, *norms.offset(index as isize));
        }
        return 1;
    }
    0
}

/// `int c2RaytoPoly(c2Ray A, const c2Poly *B, const c2x *bx_ptr, c2Raycast *out)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoPoly(
    A: c2Ray,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    out: *mut c2Raycast,
) -> c_int {
    unsafe { rayto_poly(A, B, bx_ptr, out) }
}

/// Shared implementation of `c2CastRay`.
///
/// # Safety
/// `B` must point to an object of the type selected by `typeB`.
unsafe fn cast_ray(
    A: c2Ray,
    B: *const core::ffi::c_void,
    bx: *const c2x,
    typeB: u32,
    out: *mut c2Raycast,
) -> c_int {
    unsafe {
        match typeB {
            C2_TYPE_CIRCLE => rayto_circle(A, *(B as *const c2Circle), out),
            C2_TYPE_AABB => rayto_aabb(A, *(B as *const c2AABB), out),
            C2_TYPE_CAPSULE => rayto_capsule(A, *(B as *const c2Capsule), out),
            C2_TYPE_POLY => rayto_poly(A, B as *const c2Poly, bx, out),
            _ => 0,
        }
    }
}

/// `int c2CastRay(c2Ray A, const void *B, const c2x *bx, C2_TYPE typeB, c2Raycast *out)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const core::ffi::c_void,
    bx: *const c2x,
    typeB: u32,
    out: *mut c2Raycast,
) -> c_int {
    unsafe { cast_ray(A, B, bx, typeB, out) }
}

// ---------------------------------------------------------------------------
// Public entry point (c_src/include/lib.h)
// ---------------------------------------------------------------------------

/// `int poly_ray(c2Raycast *cast1, c2Raycast *cast2)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn poly_ray(cast1: *mut c2Raycast, cast2: *mut c2Raycast) -> c_int {
    let mut hit: c_int = 0;

    let mut p = c2Poly::default();
    p.verts[0] = v(0.875f32, -11.5f32);
    p.verts[1] = v(0.875f32, 11.5f32);
    p.verts[2] = v(-0.875f32, 11.5f32);
    p.verts[3] = v(-0.875f32, -11.5f32);
    p.norms[0] = v(1.0, 0.0);
    p.norms[1] = v(0.0, 1.0);
    p.norms[2] = v(-1.0, 0.0);
    p.norms[3] = v(0.0, -1.0);
    p.count = 4;

    let ray0 = c2Ray {
        p: v(-3.869416f32, 13.0693407f32),
        d: v(1.0, 0.0),
        t: 4.0,
    };
    let ray1 = c2Ray {
        p: v(-3.869416f32, 13.0693407f32),
        d: v(0.0, -1.0),
        t: 4.0,
    };

    let pp: *const c2Poly = &p;
    unsafe {
        hit += cast_ray(
            ray0,
            pp as *const core::ffi::c_void,
            core::ptr::null(),
            C2_TYPE_POLY,
            cast1,
        );
        hit += cast_ray(
            ray1,
            pp as *const core::ffi::c_void,
            core::ptr::null(),
            C2_TYPE_POLY,
            cast2,
        ) << 1;
    }

    hit
}
