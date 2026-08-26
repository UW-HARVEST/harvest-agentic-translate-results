//! Rust translation of c_src/src/lib.c (a cute_c2 / tinyc2 derived raycast library).
//!
//! The C translation unit exports 22 public symbols; every one of them is
//! reproduced here with an identical `extern "C"` signature so that the
//! resulting cdylib is ABI compatible with `libtranslated_rust.so` built from
//! `c_src`.
//!
//! Fidelity notes:
//!  * The C source spells `abs`/`min`/`max` out as ternary macros
//!    (`(a) < 0 ? -(a) : (a)`, `(a) < (b) ? (a) : (b)`).  Those do NOT behave
//!    like `f32::abs` / `f32::min` / `f32::max` for `-0.0` and `NaN`
//!    operands, so the ternaries are transcribed literally rather than being
//!    "cleaned up" into the standard library helpers.
//!  * `c2CastRay` has a `switch` over `C2_TYPE` with no `default:` arm and no
//!    trailing `return`, so falling off the end is undefined behaviour.  This
//!    is not "fixed"; instead the behaviour of the compiled C artifact is
//!    reproduced (see `c2CastRay` below).
//!  * `c2Div`/`c2Norm` compute a reciprocal and then multiply, exactly as the
//!    C does, rather than dividing componentwise.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Public types (include/lib.h)
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
// Private types (src/lib.c)
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;`
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

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
// Literal transcriptions of the C ternary macros.
//
// These deliberately do NOT use `f32::abs`, `f32::min` or `f32::max`, because
// the C macros differ from those for signed zeros and NaNs:
//   * `-0.0f < 0` is false, so the macro yields `-0.0f` where `fabsf` yields
//     `+0.0f`.
//   * any comparison with NaN is false, so the macros propagate the *second*
//     operand (min/max) or the unmodified NaN with its sign bit intact (abs),
//     whereas the standard-library helpers pick the non-NaN operand / clear
//     the sign bit.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Order-preserving IEEE-754 primitives.
//
// IEEE addition and multiplication are commutative for every operand pair
// EXCEPT when both operands are NaN: x86 `ADDSS`/`ADDPS`/`MULSS`/`MULPS`
// return the *destination* (left-hand) operand, quieted.  LLVM treats `fadd`
// and `fmul` as freely commutable, and it does in fact swap them here -- its
// SLP vectorizer reassociates lane 1 of `c2MulmvT` into
// `a.y.y*b.y + a.y.x*b.x`, which flips the sign of the resulting NaN relative
// to the C artifact (`ffc00000` vs `7fc00000`).
//
// These helpers pin the operand order written in the C source so that NaN
// sign and payload bits are reproduced exactly.  For non-NaN operands they
// are plain IEEE ops, so ordinary results are bit-for-bit unaffected.
// ---------------------------------------------------------------------------

/// Quiet a NaN the way the x86 FP units do: set the mantissa MSB, leave the
/// sign bit and the rest of the payload alone.
#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `a + b`, selecting the left operand's NaN when both are NaN.
#[inline]
fn fadd(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a + b
    }
}

/// `a - b`, selecting the left operand's NaN when both are NaN.
#[inline]
fn fsub(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a - b
    }
}

/// `a * b`, selecting the left operand's NaN when both are NaN.
#[inline]
fn fmul(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a * b
    }
}

/// `a / b`, selecting the left operand's NaN when both are NaN.
#[inline]
fn fdiv(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a / b
    }
}

/// `#define c2Abs(a) ((a) < 0 ? -(a) : (a))`
#[inline]
fn m_abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

/// `#define c2Min(a, b) ((a) < (b) ? (a) : (b))`
#[inline]
fn m_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `#define c2Max(a, b) ((a) > (b) ? (a) : (b))`
#[inline]
fn m_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

/// ```c
/// c2v c2V(float x, float y) {
///         c2v a;
///         a.x = x;
///         a.y = y;
///         return a;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

/// ```c
/// float c2Dot(c2v a, c2v b) {
///         return a.x * b.x + a.y * b.y;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    fadd(fmul(a.x, b.x), fmul(a.y, b.y))
}

/// ```c
/// float c2Len(c2v a) {
///         return sqrtf(c2Dot(a, a));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

/// ```c
/// c2v c2Add(c2v a, c2v b) {
///         a.x += b.x;
///         a.y += b.y;
///         return a;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x = fadd(a.x, b.x);
    a.y = fadd(a.y, b.y);
    a
}

/// ```c
/// c2v c2Sub(c2v a, c2v b) {
///         a.x -= b.x;
///         a.y -= b.y;
///         return a;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x = fsub(a.x, b.x);
    a.y = fsub(a.y, b.y);
    a
}

/// ```c
/// c2v c2Mulvs(c2v a, float b) {
///         a.x *= b;
///         a.y *= b;
///         return a;
/// }
/// ```
///
/// GCC compiles this to `movsldup`/`mulps` with the *broadcast `b`* as the
/// destination register (`mulps %xmm2,%xmm0` where `xmm0 = [b, b]`), i.e. it
/// commutes the multiply.  The operand order below matches that artifact, so a
/// NaN `b` wins over a NaN `a.x`/`a.y`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x = fmul(b, a.x);
    a.y = fmul(b, a.y);
    a
}

/// ```c
/// c2v c2Div(c2v a, float b) {
///         return c2Mulvs(a, 1.0f / b);
/// }
/// ```
///
/// Note: reciprocal-then-multiply, not componentwise division.
#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, fdiv(1.0f32, b))
}

/// ```c
/// c2v c2Norm(c2v a) {
///         return c2Div(a, c2Len(a));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

/// ```c
/// c2v c2Minv(c2v a, c2v b) {
///         return c2V(((a.x) < (b.x) ? (a.x) : (b.x)),
///                         ((a.y) < (b.y) ? (a.y) : (b.y)));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(m_min(a.x, b.x), m_min(a.y, b.y))
}

/// ```c
/// c2v c2Maxv(c2v a, c2v b) {
///         return c2V(((a.x) > (b.x) ? (a.x) : (b.x)),
///                         ((a.y) > (b.y) ? (a.y) : (b.y)));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(m_max(a.x, b.x), m_max(a.y, b.y))
}

/// ```c
/// c2v c2Skew(c2v a) {
///         c2v b;
///         b.x = -a.y;
///         b.y = a.x;
///         return b;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

/// ```c
/// c2v c2Absv(c2v a) {
///         return c2V(((a.x) < 0 ? -(a.x) : (a.x)), ((a.y) < 0 ? -(a.y) : (a.y)));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(m_abs(a.x), m_abs(a.y))
}

/// ```c
/// c2v c2CCW90(c2v a) {
///         c2v b;
///         b.x = a.y;
///         b.y = -a.x;
///         return b;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

/// ```c
/// c2v c2MulmvT(c2m a, c2v b) {
///         c2v c;
///         c.x = a.x.x * b.x + a.x.y * b.y;
///         c.y = a.y.x * b.x + a.y.y * b.y;
///         return c;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    c2v {
        x: fadd(fmul(a.x.x, b.x), fmul(a.x.y, b.y)),
        y: fadd(fmul(a.y.x, b.x), fmul(a.y.y, b.y)),
    }
}

// ---------------------------------------------------------------------------
// Overlap tests
// ---------------------------------------------------------------------------

/// ```c
/// int c2AABBtoAABB(c2AABB A, c2AABB B) {
///         int d0 = B.max.x < A.min.x;
///         int d1 = A.max.x < B.min.x;
///         int d2 = B.max.y < A.min.y;
///         int d3 = A.max.y < B.min.y;
///         return !(d0 | d1 | d2 | d3);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0: c_int = (B.max.x < A.min.x) as c_int;
    let d1: c_int = (A.max.x < B.min.x) as c_int;
    let d2: c_int = (B.max.y < A.min.y) as c_int;
    let d3: c_int = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// ```c
/// int c2AABBtoPoint(c2AABB A, c2v B) {
///         int d0 = B.x < A.min.x;
///         int d1 = B.y < A.min.y;
///         int d2 = B.x > A.max.x;
///         int d3 = B.y > A.max.y;
///         return !(d0 | d1 | d2 | d3);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    let d0: c_int = (B.x < A.min.x) as c_int;
    let d1: c_int = (B.y < A.min.y) as c_int;
    let d2: c_int = (B.x > A.max.x) as c_int;
    let d3: c_int = (B.y > A.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// ```c
/// int c2CircleToPoint(c2Circle A, c2v B) {
///         c2v n = c2Sub(A.p, B);
///         float d2 = c2Dot(n, n);
///         return d2 < A.r * A.r;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    let n = c2Sub(A.p, B);
    let d2 = c2Dot(n, n);
    (d2 < fmul(A.r, A.r)) as c_int
}

// ---------------------------------------------------------------------------
// `static inline` plane helpers (not exported by the C library)
// ---------------------------------------------------------------------------

/// ```c
/// static inline float c2SignedDistPointToPlane_OneDimensional(float p, float n,
///                 float d) {
///         return p * n - d * n;
/// }
/// ```
///
/// `n` is always the literal `1.0f` or `-1.0f` at the eight call sites in
/// `c2RaytoAABB`, and GCC constant-folds the two multiplies away entirely:
///
/// * `n == 1.0f`  becomes `subss` with `p` as the destination (`p - d`)
/// * `n == -1.0f` becomes `subss` with `d` as the destination (`d - p`)
///
/// The second form is *not* the same as evaluating `(-p) - (-d)` when both
/// operands are NaN (the destination operand wins, and `-p` would have had its
/// sign bit flipped), so the two specializations are transcribed separately.
#[inline]
fn c2SignedDist_pos1(p: f32, d: f32) -> f32 {
    // p * 1.0f - d * 1.0f  ==>  p - d
    fsub(p, d)
}

/// See [`c2SignedDist_pos1`]: the `n == -1.0f` specialization.
#[inline]
fn c2SignedDist_neg1(p: f32, d: f32) -> f32 {
    // p * -1.0f - d * -1.0f  ==>  d - p
    fsub(d, p)
}

/// ```c
/// static inline float c2RayToPlane_OneDimensional(float da, float db) {
///         if (da < 0)
///                 return 0;
///         else if (da * db > 0)
///                 return 1.0f;
///         else {
///                 float d = da - db;
///                 if (d != 0)
///                         return da / d;
///                 else
///                         return 0;
///         }
/// }
/// ```
#[inline]
fn c2RayToPlane_OneDimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if fmul(db, da) > 0.0 {
        // GCC emits `mulss %xmm5,%xmm6` with `db` as the destination.
        1.0f32
    } else {
        let d = fsub(da, db);
        if d != 0.0 { fdiv(da, d) } else { 0.0 }
    }
}

// ---------------------------------------------------------------------------
// Raycasts
// ---------------------------------------------------------------------------

/// ```c
/// int c2RaytoCircle(c2Ray A, c2Circle B, c2Raycast *out) {
///         c2v p = B.p;
///         c2v m = c2Sub(A.p, p);
///         float c = c2Dot(m, m) - B.r * B.r;
///         float b = c2Dot(m, A.d);
///         float disc = b * b - c;
///         if (disc < 0)
///                 return 0;
///         float t = -b - sqrtf(disc);
///         if (t >= 0 && t <= A.t) {
///                 out->t = t;
///                 c2v impact = c2Add(A.p, c2Mulvs(A.d, t));
///                 out->n = c2Norm(c2Sub(impact, p));
///                 return 1;
///         }
///         return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    let p = B.p;
    let m = c2Sub(A.p, p);
    let c = fsub(c2Dot(m, m), fmul(B.r, B.r));
    let b = c2Dot(m, A.d);
    let disc = fsub(fmul(b, b), c);
    if disc < 0.0 {
        return 0;
    }
    let t = fsub(-b, disc.sqrt());
    if t >= 0.0 && t <= A.t {
        unsafe {
            (*out).t = t;
            let impact = c2Add(A.p, c2Mulvs(A.d, t));
            (*out).n = c2Norm(c2Sub(impact, p));
        }
        return 1;
    }
    0
}

/// ```c
/// int c2RaytoAABB(c2Ray A, c2AABB B, c2Raycast *out) { ... }
/// ```
///
/// See `c_src/src/lib.c:139` — the structure (including the `d > 0` early out
/// and the four-way `>=` cascade that picks the exit face) is preserved
/// exactly, as is the `(float)hitN * tN` masking which propagates NaN when a
/// `tN` is NaN even though `hitN` is 0.
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
    let half_extents = c2Mulvs(c2Sub(B.max, B.min), 0.5f32);
    let center_of_b_box = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
    let d = fsub(
        m_abs(c2Dot(n, c2Sub(p0, center_of_b_box))),
        c2Dot(abs_n, half_extents),
    );
    if d > 0.0 {
        return 0;
    }
    let da0 = c2SignedDist_neg1(p0.x, B.min.x);
    let db0 = c2SignedDist_neg1(p1.x, B.min.x);
    let da1 = c2SignedDist_pos1(p0.x, B.max.x);
    let db1 = c2SignedDist_pos1(p1.x, B.max.x);
    let da2 = c2SignedDist_neg1(p0.y, B.min.y);
    let db2 = c2SignedDist_neg1(p1.y, B.min.y);
    let da3 = c2SignedDist_pos1(p0.y, B.max.y);
    let db3 = c2SignedDist_pos1(p1.y, B.max.y);
    let mut t0 = c2RayToPlane_OneDimensional(da0, db0);
    let mut t1 = c2RayToPlane_OneDimensional(da1, db1);
    let mut t2 = c2RayToPlane_OneDimensional(da2, db2);
    let mut t3 = c2RayToPlane_OneDimensional(da3, db3);
    let hit0: c_int = (t0 <= 1.0f32) as c_int;
    let hit1: c_int = (t1 <= 1.0f32) as c_int;
    let hit2: c_int = (t2 <= 1.0f32) as c_int;
    let hit3: c_int = (t3 <= 1.0f32) as c_int;
    let hit: c_int = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = fmul(hit0 as f32, t0);
        t1 = fmul(hit1 as f32, t1);
        t2 = fmul(hit2 as f32, t2);
        t3 = fmul(hit3 as f32, t3);
        unsafe {
            if t0 >= t1 && t0 >= t2 && t0 >= t3 {
                (*out).t = fmul(t0, A.t);
                (*out).n = c2V(-1.0, 0.0);
            } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
                (*out).t = fmul(t1, A.t);
                (*out).n = c2V(1.0, 0.0);
            } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
                (*out).t = fmul(t2, A.t);
                (*out).n = c2V(0.0, -1.0);
            } else {
                (*out).t = fmul(t3, A.t);
                (*out).n = c2V(0.0, 1.0);
            }
        }
        1
    } else {
        0
    }
}

/// ```c
/// int c2RaytoCapsule(c2Ray A, c2Capsule B, c2Raycast *out) { ... }
/// ```
///
/// See `c_src/src/lib.c:231`.  Note that `out->n` / `out->t` are written
/// *before* the early-out overlap tests, so `out` is mutated even on the
/// `return 1` "already inside" paths and on some `return 0` paths — that
/// side effect is preserved.
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
    if fmul(yAe.x, yAp.x) < 0.0 || m_min(m_abs(yAe.x), m_abs(yAp.x)) < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if m_abs(yAp.x) < B.r {
            if yAp.y < 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            } else {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = fsub(yAe.x, yAp.x);
            let t = fdiv(fsub(c, yAp.x), d);
            // GCC emits `addss %xmm6,%xmm0` with the *product* as the
            // destination, i.e. it commutes this addition.
            let y = fadd(fmul(fsub(yAe.y, yAp.y), t), yAp.y);
            if y <= 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            }
            if y >= yBb.y {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                    (*out).t = fmul(t, A.t);
                }
                return 1;
            }
        }
    }
    0
}

/// ```c
/// int c2CastRay(c2Ray A, const void *B, C2_TYPE typeB, c2Raycast *out) {
///         switch (typeB) {
///                 case C2_TYPE_CIRCLE:  return c2RaytoCircle(A, *(c2Circle *)B, out);
///                 case C2_TYPE_AABB:    return c2RaytoAABB(A, *(c2AABB *)B, out);
///                 case C2_TYPE_CAPSULE: return c2RaytoCapsule(A, *(c2Capsule *)B, out);
///                                       return 0;
///         }
/// }
/// ```
///
/// The `switch` has no `default:` arm and control can fall off the end of the
/// function, which is undefined behaviour in C.  This bug is intentionally NOT
/// fixed.  The `-O3` artifact built from `c_src` reaches its epilogue with
/// `%rax` still holding the value moved there in the prologue (`mov %rdi,%rax`,
/// i.e. the `B` argument, since the 20-byte `c2Ray A` is passed in memory), so
/// an out-of-range `typeB` yields the low 32 bits of `B` as the return value.
/// That is reproduced here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    unsafe {
        match typeB {
            C2_TYPE_CIRCLE => c2RaytoCircle(A, *(B as *const c2Circle), out),
            C2_TYPE_AABB => c2RaytoAABB(A, *(B as *const c2AABB), out),
            C2_TYPE_CAPSULE => c2RaytoCapsule(A, *(B as *const c2Capsule), out),
            // Fall off the end of the C `switch`: the return value is the
            // stale `%rax` from the prologue, which holds `B`.
            _ => (B as usize as u32) as c_int,
        }
    }
}

/// ```c
/// int gen_ray(c2Raycast *cast1, c2Raycast *cast2, c2Raycast *cast3,
///                 float mp_x, float mp_y, float r_p_x, float r_p_y,
///                 float c_p_x, float c_p_y, float c_r,
///                 float cap_a_x, float cap_a_y, float cap_b_x, float cap_b_y, float cap_r,
///                 float bb_min_x, float bb_min_y, float bb_max_x, float bb_max_y) { ... }
/// ```
///
/// See `c_src/src/lib.c:306`.  The circle result lands in bit 0, the capsule
/// result in bit 1 and the AABB result in bit 2 of the return value.
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
    let mut hit: c_int = 0;

    let mp = c2V(mp_x, mp_y);

    let mut ray = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 0.0, y: 0.0 },
        t: 0.0,
    };
    ray.p = c2V(r_p_x, r_p_y);
    ray.d = c2Norm(c2Sub(mp, ray.p));
    ray.t = fsub(c2Dot(mp, ray.d), c2Dot(ray.p, ray.d));

    let mut c = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    c.p = c2V(c_p_x, c_p_y);
    c.r = c_r;

    hit += unsafe {
        c2CastRay(
            ray,
            (&raw const c) as *const c_void,
            C2_TYPE_CIRCLE,
            cast1,
        )
    };

    let mut cap = c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    cap.a = c2V(cap_a_x, cap_a_y);
    cap.b = c2V(cap_b_x, cap_b_y);
    cap.r = cap_r;

    hit += unsafe {
        c2CastRay(
            ray,
            (&raw const cap) as *const c_void,
            C2_TYPE_CAPSULE,
            cast2,
        )
    } << 1;

    let mut bb = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 0.0, y: 0.0 },
    };
    bb.min = c2V(bb_min_x, bb_min_y);
    bb.max = c2V(bb_max_x, bb_max_y);

    hit += unsafe { c2CastRay(ray, (&raw const bb) as *const c_void, C2_TYPE_AABB, cast3) } << 2;

    hit
}
