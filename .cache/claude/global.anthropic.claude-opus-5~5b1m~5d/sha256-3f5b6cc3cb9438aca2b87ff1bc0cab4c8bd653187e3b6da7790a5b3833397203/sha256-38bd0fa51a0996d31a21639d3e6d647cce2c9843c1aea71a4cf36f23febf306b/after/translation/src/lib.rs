//! Rust translation of the C library in `c_src/` (a trimmed-down `cute_c2`
//! style 2D raycast library).
//!
//! Every public symbol exported by the C shared object is re-exported here with
//! the identical name, C ABI and bit-for-bit identical floating-point
//! behaviour.  The original C source is *not* corrected: its quirks are
//! reproduced exactly, including
//!
//!   * `c2Div` multiplying by `1.0f / b` instead of dividing,
//!   * `fabsf`/`fminf`/`fmaxf` spelled as ternaries (so `-0.0` and `-NaN` keep
//!     their sign, unlike the real libm functions),
//!   * the missing `default:`/trailing `return` in `c2CastRay`.
//!
//! ## Why the `f*` arithmetic helpers exist
//!
//! `mulss`/`addss` return the *destination* operand when both operands are
//! NaN, and LLVM is free to commute the operands of `fmul`/`fadd` (it considers
//! NaN payload/sign propagation non-deterministic).  That makes plain `a * b`
//! observably different from C for inputs such as `±inf` or NaN.  The helpers
//! below pin each operand to the role it has in the C source so the emitted
//! instruction sequence matches the reference build operand-for-operand.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Order-preserving scalar float arithmetic
// ---------------------------------------------------------------------------

macro_rules! ordered_binop {
    ($name:ident, $mnemonic:literal, $fallback:expr) => {
        #[inline(always)]
        fn $name(a: f32, b: f32) -> f32 {
            #[cfg(target_arch = "x86_64")]
            {
                let mut r = a;
                // SAFETY: a pure, memory-free SSE scalar instruction on two
                // `f32` values held in `xmm` registers.
                unsafe {
                    core::arch::asm!(
                        concat!($mnemonic, " {0}, {1}"),
                        inout(xmm_reg) r,
                        in(xmm_reg) b,
                        options(pure, nomem, nostack, preserves_flags),
                    );
                }
                r
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                let f: fn(f32, f32) -> f32 = $fallback;
                f(a, b)
            }
        }
    };
}

ordered_binop!(fadd, "addss", |a, b| a + b);
ordered_binop!(fsub, "subss", |a, b| a - b);
ordered_binop!(fmul, "mulss", |a, b| a * b);
ordered_binop!(fdiv, "divss", |a, b| a / b);

/// `sqrtf(a)` — identical result to glibc's `sqrtf` (both are a single
/// `sqrtss`, including the `-NaN` produced for negative arguments).
#[inline(always)]
fn fsqrt(a: f32) -> f32 {
    a.sqrt()
}

/// Reproduces the C source's inlined `fabsf` idiom `((a) < 0 ? -(a) : (a))`.
///
/// Differs from `f32::abs` for `-0.0` and `-NaN` (which are returned
/// unchanged, because `x < 0` is false for both) — the C behaviour is kept.
#[inline(always)]
fn abs_ternary(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

/// Reproduces `((a) < (b) ? (a) : (b))`.
#[inline(always)]
fn min_ternary(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// Reproduces `((a) > (b) ? (a) : (b))`.
#[inline(always)]
fn max_ternary(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

// ---------------------------------------------------------------------------
// Public types (include/lib.h)
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
// Private types (src/lib.c)
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;`
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

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
// Internal implementations.  The exported `extern "C"` wrappers further down
// are thin shims over these, so the internal call graph mirrors the C one.
// ---------------------------------------------------------------------------

/// `c2V`
#[inline(always)]
fn c2v_new(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

/// `c2Dot`
#[inline(always)]
fn dot(a: c2v, b: c2v) -> f32 {
    fadd(fmul(a.x, b.x), fmul(a.y, b.y))
}

/// `c2Len`
#[inline(always)]
fn len(a: c2v) -> f32 {
    fsqrt(dot(a, a))
}

/// `c2Add`
#[inline(always)]
fn add(mut a: c2v, b: c2v) -> c2v {
    a.x = fadd(a.x, b.x);
    a.y = fadd(a.y, b.y);
    a
}

/// `c2Sub`
#[inline(always)]
fn sub(mut a: c2v, b: c2v) -> c2v {
    a.x = fsub(a.x, b.x);
    a.y = fsub(a.y, b.y);
    a
}

/// `c2Mulvs`
///
/// The reference build keeps `a.x` as the `mulss` destination for the `x` lane
/// but uses `b` as the destination for the `y` lane, which is observable when
/// both operands are NaN (the destination wins).  Mirrored here.
#[inline(always)]
fn mulvs(mut a: c2v, b: f32) -> c2v {
    a.x = fmul(a.x, b);
    a.y = fmul(b, a.y);
    a
}

/// `c2Div` — note the reciprocal multiply, which is observable in the low bits.
#[inline(always)]
fn div(a: c2v, b: f32) -> c2v {
    mulvs(a, fdiv(1.0f32, b))
}

/// `c2Norm`
#[inline(always)]
fn norm(a: c2v) -> c2v {
    div(a, len(a))
}

/// `c2Minv`
#[inline(always)]
fn minv(a: c2v, b: c2v) -> c2v {
    c2v_new(min_ternary(a.x, b.x), min_ternary(a.y, b.y))
}

/// `c2Maxv`
#[inline(always)]
fn maxv(a: c2v, b: c2v) -> c2v {
    c2v_new(max_ternary(a.x, b.x), max_ternary(a.y, b.y))
}

/// `c2Skew`
#[inline(always)]
fn skew(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = -a.y;
    b.y = a.x;
    b
}

/// `c2Absv`
#[inline(always)]
fn absv(a: c2v) -> c2v {
    c2v_new(abs_ternary(a.x), abs_ternary(a.y))
}

/// `c2CCW90`
#[inline(always)]
fn ccw90(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = a.y;
    b.y = -a.x;
    b
}

/// `c2MulmvT`
#[inline(always)]
fn mulmv_t(a: c2m, b: c2v) -> c2v {
    let mut c = c2v { x: 0.0, y: 0.0 };
    c.x = fadd(fmul(a.x.x, b.x), fmul(a.x.y, b.y));
    c.y = fadd(fmul(a.y.x, b.x), fmul(a.y.y, b.y));
    c
}

/// `c2AABBtoAABB`
#[inline(always)]
fn aabb_to_aabb(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// `c2AABBtoPoint`
#[inline(always)]
fn aabb_to_point(A: c2AABB, B: c2v) -> c_int {
    let d0 = (B.x < A.min.x) as c_int;
    let d1 = (B.y < A.min.y) as c_int;
    let d2 = (B.x > A.max.x) as c_int;
    let d3 = (B.y > A.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// `c2CircleToPoint`
#[inline(always)]
fn circle_to_point(A: c2Circle, B: c2v) -> c_int {
    let n = sub(A.p, B);
    let d2 = dot(n, n);
    (d2 < fmul(A.r, A.r)) as c_int
}

/// `static inline float c2SignedDistPointToPlane_OneDimensional(float p, float n, float d)`
#[inline(always)]
fn signed_dist_point_to_plane_one_dimensional(p: f32, n: f32, d: f32) -> f32 {
    fsub(fmul(p, n), fmul(d, n))
}

/// `static inline float c2RayToPlane_OneDimensional(float da, float db)`
#[inline(always)]
fn ray_to_plane_one_dimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if fmul(da, db) > 0.0 {
        1.0f32
    } else {
        let d = fsub(da, db);
        if d != 0.0 { fdiv(da, d) } else { 0.0 }
    }
}

/// `c2RaytoCircle`
///
/// # Safety
/// `out` must be a valid, writable pointer to a `c2Raycast` (the C code
/// dereferences it unconditionally on the hit path).
unsafe fn ray_to_circle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    let p = B.p;
    let m = sub(A.p, p);
    let c = fsub(dot(m, m), fmul(B.r, B.r));
    let b = dot(m, A.d);
    let disc = fsub(fmul(b, b), c);
    if disc < 0.0 {
        return 0;
    }
    let t = fsub(-b, fsqrt(disc));
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

/// `c2RaytoAABB`
///
/// # Safety
/// `out` must be a valid, writable pointer to a `c2Raycast`.
unsafe fn ray_to_aabb(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
    let p0 = A.p;
    let p1 = add(A.p, mulvs(A.d, A.t));
    let mut a_box = c2AABB::default();
    a_box.min = minv(p0, p1);
    a_box.max = maxv(p0, p1);
    if aabb_to_aabb(a_box, B) == 0 {
        return 0;
    }
    let ab = sub(p1, p0);
    let n = skew(ab);
    let abs_n = absv(n);
    let half_extents = mulvs(sub(B.max, B.min), 0.5f32);
    let center_of_b_box = mulvs(add(B.min, B.max), 0.5f32);
    let d = fsub(
        abs_ternary(dot(n, sub(p0, center_of_b_box))),
        dot(abs_n, half_extents),
    );
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
        t0 = fmul(hit0 as f32, t0);
        t1 = fmul(hit1 as f32, t1);
        t2 = fmul(hit2 as f32, t2);
        t3 = fmul(hit3 as f32, t3);
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            unsafe {
                (*out).t = fmul(t0, A.t);
                (*out).n = c2v_new(-1.0, 0.0);
            }
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            unsafe {
                (*out).t = fmul(t1, A.t);
                (*out).n = c2v_new(1.0, 0.0);
            }
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            unsafe {
                (*out).t = fmul(t2, A.t);
                (*out).n = c2v_new(0.0, -1.0);
            }
        } else {
            unsafe {
                (*out).t = fmul(t3, A.t);
                (*out).n = c2v_new(0.0, 1.0);
            }
        }
        1
    } else {
        0
    }
}

/// `c2RaytoCapsule`
///
/// # Safety
/// `out` must be a valid, writable pointer to a `c2Raycast`.
unsafe fn ray_to_capsule(A: c2Ray, B: c2Capsule, out: *mut c2Raycast) -> c_int {
    let mut M = c2m::default();
    M.y = norm(sub(B.b, B.a));
    M.x = ccw90(M.y);
    let cap_n = sub(B.b, B.a);
    let yBb = mulmv_t(M, cap_n);
    let yAp = mulmv_t(M, sub(A.p, B.a));
    let yAd = mulmv_t(M, A.d);
    let yAe = add(yAp, mulvs(yAd, A.t));
    let mut capsule_bb = c2AABB::default();
    capsule_bb.min = c2v_new(-B.r, 0.0);
    capsule_bb.max = c2v_new(B.r, yBb.y);
    unsafe {
        (*out).n = norm(cap_n);
        (*out).t = 0.0;
    }
    if aabb_to_point(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let mut capsule_a = c2Circle::default();
        let mut capsule_b = c2Circle::default();
        capsule_a.p = B.a;
        capsule_a.r = B.r;
        capsule_b.p = B.b;
        capsule_b.r = B.r;
        if circle_to_point(capsule_a, A.p) != 0 {
            return 1;
        } else if circle_to_point(capsule_b, A.p) != 0 {
            return 1;
        }
    }
    if fmul(yAe.x, yAp.x) < 0.0
        || min_ternary(abs_ternary(yAe.x), abs_ternary(yAp.x)) < B.r
    {
        let mut Ca = c2Circle::default();
        let mut Cb = c2Circle::default();
        Ca.p = B.a;
        Ca.r = B.r;
        Cb.p = B.b;
        Cb.r = B.r;
        if abs_ternary(yAp.x) < B.r {
            if yAp.y < 0.0 {
                return unsafe { ray_to_circle(A, Ca, out) };
            } else {
                return unsafe { ray_to_circle(A, Cb, out) };
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = fsub(yAe.x, yAp.x);
            let t = fdiv(fsub(c, yAp.x), d);
            let y = fadd(yAp.y, fmul(fsub(yAe.y, yAp.y), t));
            if y <= 0.0 {
                return unsafe { ray_to_circle(A, Ca, out) };
            }
            if y >= yBb.y {
                return unsafe { ray_to_circle(A, Cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { M.x } else { skew(M.y) };
                    (*out).t = fmul(t, A.t);
                }
                return 1;
            }
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Exported C ABI surface
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v_new(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    dot(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    len(a)
}

/// The reference build emits `addss` with `b` as the destination for both
/// lanes of the standalone `c2Add`, which is observable when both addends are
/// NaN (the destination operand wins).  Mirrored here.
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x = fadd(b.x, a.x);
    a.y = fadd(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    sub(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    mulvs(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    div(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    norm(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    minv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    maxv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    skew(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    absv(a)
}

/// # Safety
/// `out` must point to a writable `c2Raycast`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    unsafe { ray_to_circle(A, B, out) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    aabb_to_aabb(A, B)
}

/// # Safety
/// `out` must point to a writable `c2Raycast`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
    unsafe { ray_to_aabb(A, B, out) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    ccw90(a)
}

/// In the standalone `c2MulmvT` the reference build happens to use `b` as the
/// `mulss` destination for the first row and `a.y` for the second; that is
/// observable when both factors are NaN.  Mirrored here.
#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    let mut c = c2v { x: 0.0, y: 0.0 };
    c.x = fadd(fmul(b.x, a.x.x), fmul(b.y, a.x.y));
    c.y = fadd(fmul(a.y.x, b.x), fmul(a.y.y, b.y));
    c
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    aabb_to_point(A, B)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    circle_to_point(A, B)
}

/// # Safety
/// `out` must point to a writable `c2Raycast`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: *mut c2Raycast) -> c_int {
    unsafe { ray_to_capsule(A, B, out) }
}

/// # Safety
/// `B` must point to a `c2Circle`, `c2AABB` or `c2Capsule` matching `typeB`,
/// and `out` must point to a writable `c2Raycast`.
///
/// The C original has neither a `default:` label nor a `return` after the
/// `switch`, so an out-of-range `typeB` falls off the end of the function.  On
/// the reference build that leaves `eax` holding the low 32 bits of the `B`
/// pointer; that (undefined) behaviour is mirrored rather than "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    unsafe {
        match typeB {
            C2_TYPE_CIRCLE => ray_to_circle(A, *(B as *const c2Circle), out),
            C2_TYPE_AABB => ray_to_aabb(A, *(B as *const c2AABB), out),
            C2_TYPE_CAPSULE => ray_to_capsule(A, *(B as *const c2Capsule), out),
            // Falls off the end of the C `switch`: garbage return value.
            _ => (B as usize as u32) as c_int,
        }
    }
}

/// # Safety
/// `cast1`, `cast2` and `cast3` must point to writable `c2Raycast` values.
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

    let mp = c2v_new(mp_x, mp_y);

    let mut ray = c2Ray::default();
    ray.p = c2v_new(r_p_x, r_p_y);
    ray.d = norm(sub(mp, ray.p));
    ray.t = fsub(dot(mp, ray.d), dot(ray.p, ray.d));

    let mut c = c2Circle::default();
    c.p = c2v_new(c_p_x, c_p_y);
    c.r = c_r;

    hit = hit.wrapping_add(unsafe {
        c2CastRay(ray, (&raw const c) as *const c_void, C2_TYPE_CIRCLE, cast1)
    });

    let mut cap = c2Capsule::default();
    cap.a = c2v_new(cap_a_x, cap_a_y);
    cap.b = c2v_new(cap_b_x, cap_b_y);
    cap.r = cap_r;

    hit = hit.wrapping_add(
        unsafe {
            c2CastRay(
                ray,
                (&raw const cap) as *const c_void,
                C2_TYPE_CAPSULE,
                cast2,
            )
        } << 1,
    );

    let mut bb = c2AABB::default();
    bb.min = c2v_new(bb_min_x, bb_min_y);
    bb.max = c2v_new(bb_max_x, bb_max_y);

    hit = hit.wrapping_add(
        unsafe { c2CastRay(ray, (&raw const bb) as *const c_void, C2_TYPE_AABB, cast3) } << 2,
    );

    hit
}
