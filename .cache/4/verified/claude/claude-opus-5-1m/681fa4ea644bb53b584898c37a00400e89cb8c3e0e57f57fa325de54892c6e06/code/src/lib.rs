//! Rust translation of the C library in `c_src/` (cute_c2-style 2D ray casting).
//!
//! Every public symbol exported by the C shared library is reproduced here with
//! the identical linker name, C ABI signature and bit-exact behaviour.
//!
//! Notes on fidelity:
//!  * All arithmetic is performed in `f32` (single precision), exactly like the C
//!    code which is compiled for SSE-based x86-64 (no x87 excess precision and
//!    no FMA contraction on the base target).
//!  * The C source expands min/max/abs as ternary expressions, whose NaN and
//!    signed-zero behaviour differs from `f32::min`/`f32::max`/`f32::abs`.
//!    The ternaries are reproduced literally so that e.g. `c2Absv(-0.0)` still
//!    returns `-0.0` and NaN operands propagate identically.
//!  * `c2Div` multiplies by the reciprocal (`1.0f / b`) rather than dividing,
//!    matching the C source.
//!  * Every `+`/`-`/`*`/`/` goes through the `fadd`/`fsub`/`fmul`/`fdiv` helpers
//!    below, which pin down *which operand is the SSE destination register*.
//!    That is observable when both operands are NaN (`ADDSS/MULSS` then return
//!    the destination operand, quieted), and gcc does not always pick the
//!    left-hand operand of the C expression.  The orders were read off the
//!    reference `.so`'s disassembly; see `CONFIGS.md` for details.
//!  * No bugs are fixed: the write of `out->n` / `out->t` before the early-out
//!    checks in `c2RaytoCapsule`, the recomputation of the dot product in
//!    `c2RaytoAABB`, and the dead `return 0;` in `c2CastRay` are all preserved.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};

/* -------------------------------------------------------------------------- */
/* Public types (from include/lib.h)                                          */
/* -------------------------------------------------------------------------- */

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

/* -------------------------------------------------------------------------- */
/* Private types (from src/lib.c)                                             */
/* -------------------------------------------------------------------------- */

// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

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

/* -------------------------------------------------------------------------- */
/* Bit-exact x86 SSE scalar arithmetic helpers                                */
/* -------------------------------------------------------------------------- */
//
// `ADDSS/SUBSS/MULSS/DIVSS dest, src` return **`dest` quieted** when *both*
// operands are NaN (Intel SDM, "Operating on NaNs"); with at most one NaN the
// result is that NaN quieted regardless of operand position.  The C compiler's
// choice of which operand lands in `dest` is therefore observable, and it is
// *not* always the left-hand operand of the C expression (gcc freely commutes
// `+` and `*`).  Rust/LLVM commutes differently, so every add/mul below is
// written through these helpers with the operand order taken from the
// disassembly of the reference `.so` (`objdump -d`).  For `-` and `/` the
// destination is always the left operand in both compilers, but the helpers are
// used there too so that every FP operation states its `dest` explicitly.

/// Quiet a NaN the way the x86 FPU does (set the MSB of the significand).
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `ADDSS x, y` — `x` is the destination operand.
#[inline(always)]
fn fadd(x: f32, y: f32) -> f32 {
    if x.is_nan() && y.is_nan() { quiet(x) } else { x + y }
}

/// `SUBSS x, y` — `x` is the destination operand.
#[inline(always)]
fn fsub(x: f32, y: f32) -> f32 {
    if x.is_nan() && y.is_nan() { quiet(x) } else { x - y }
}

/// `MULSS x, y` — `x` is the destination operand.
#[inline(always)]
fn fmul(x: f32, y: f32) -> f32 {
    if x.is_nan() && y.is_nan() { quiet(x) } else { x * y }
}

/// `DIVSS x, y` — `x` is the destination operand.
#[inline(always)]
fn fdiv(x: f32, y: f32) -> f32 {
    if x.is_nan() && y.is_nan() { quiet(x) } else { x / y }
}

/* -------------------------------------------------------------------------- */
/* Helpers reproducing the C ternary expansions verbatim                      */
/* -------------------------------------------------------------------------- */

/// `(a) < 0 ? -(a) : (a)`
#[inline(always)]
fn tern_abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

/// `(a) < (b) ? (a) : (b)`
#[inline(always)]
fn tern_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `(a) > (b) ? (a) : (b)`
#[inline(always)]
fn tern_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/* -------------------------------------------------------------------------- */
/* Vector math                                                                */
/* -------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // C: `return a.x * b.x + a.y * b.y;`
    //   mulss %xmm0,%xmm1   ; xmm1 = a.x  <- dest        (p1 = a.x * b.x)
    //   mulss %xmm2,%xmm0   ; xmm0 = b.y  <- dest        (p2 = b.y * a.y)
    //   addss %xmm1,%xmm0   ; xmm0 = p2   <- dest        (p2 + p1)
    let p1 = fmul(a.x, b.x);
    let p2 = fmul(b.y, a.y);
    fadd(p2, p1)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    // C: `return sqrtf(c2Dot(a, a));` — glibc's `sqrtf` wrapper only diverges
    // from `SQRTSS` for negative arguments (unreachable here: dot(a,a) >= 0 or
    // NaN), and for NaN both quiet the payload identically.
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    // C: `a.x += b.x; a.y += b.y;`
    //   addss %xmm1,%xmm0   ; xmm0 = b.x <- dest  =>  b.x + a.x
    a.x = fadd(b.x, a.x);
    a.y = fadd(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    // C: `a.x -= b.x; a.y -= b.y;`  (subss %xmm1,%xmm0 with xmm0 = a.x)
    a.x = fsub(a.x, b.x);
    a.y = fsub(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    // C: `a.x *= b; a.y *= b;`  (mulss -0xc(%rbp),%xmm0 with xmm0 = a.x, so the
    // vector component is the destination — LLVM commutes this, hence `fmul`.)
    a.x = fmul(a.x, b);
    a.y = fmul(a.y, b);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    // C: `return c2Mulvs(a, 1.0f / b);`  (divss -0xc(%rbp),%xmm0, xmm0 = 1.0f)
    c2Mulvs(a, fdiv(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(tern_min(a.x, b.x), tern_min(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(tern_max(a.x, b.x), tern_max(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(tern_abs(a.x), tern_abs(a.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    // C: `c.x = a.x.x * b.x + a.x.y * b.y;` etc.  Same gcc shape as c2Dot:
    //   mulss %xmm0,%xmm1  ; dest = a.?.x   (p1)
    //   mulss %xmm2,%xmm0  ; dest = b.y     (p2)
    //   addss %xmm1,%xmm0  ; dest = p2      (p2 + p1)
    let x = {
        let p1 = fmul(a.x.x, b.x);
        let p2 = fmul(b.y, a.x.y);
        fadd(p2, p1)
    };
    let y = {
        let p1 = fmul(a.y.x, b.x);
        let p2 = fmul(b.y, a.y.y);
        fadd(p2, p1)
    };
    c2v { x, y }
}

/* -------------------------------------------------------------------------- */
/* Overlap tests                                                              */
/* -------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0: c_int = (B.max.x < A.min.x) as c_int;
    let d1: c_int = (A.max.x < B.min.x) as c_int;
    let d2: c_int = (B.max.y < A.min.y) as c_int;
    let d3: c_int = (A.max.y < B.min.y) as c_int;
    // C: return !(d0 | d1 | d2 | d3);
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    let d0: c_int = (B.x < A.min.x) as c_int;
    let d1: c_int = (B.y < A.min.y) as c_int;
    let d2: c_int = (B.x > A.max.x) as c_int;
    let d3: c_int = (B.y > A.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    let n = c2Sub(A.p, B);
    let d2 = c2Dot(n, n);
    (d2 < fmul(A.r, A.r)) as c_int
}

/* -------------------------------------------------------------------------- */
/* Ray casts                                                                  */
/* -------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    let p = B.p;
    let m = c2Sub(A.p, p);
    // `mulss %xmm0,%xmm1` (B.r*B.r) then `subss %xmm1,%xmm0` with xmm0 = dot
    let c = fsub(c2Dot(m, m), fmul(B.r, B.r));
    let b = c2Dot(m, A.d);
    // `mulss %xmm0,%xmm0` (b*b) then `subss -0x14(%rbp),%xmm0`
    let disc = fsub(fmul(b, b), c);
    if disc < 0.0 {
        return 0;
    }
    // `xorps` sign flip on b, then `subss %xmm1,%xmm0` with xmm0 = -b
    let t = fsub(-b, disc.sqrt());
    if t >= 0.0 && t <= A.t {
        unsafe { (*out).t = t };
        let impact = c2Add(A.p, c2Mulvs(A.d, t));
        unsafe { (*out).n = c2Norm(c2Sub(impact, p)) };
        return 1;
    }
    0
}

/// `static inline float c2SignedDistPointToPlane_OneDimensional(float p, float n, float d)`
#[inline(always)]
fn c2SignedDistPointToPlane_OneDimensional(p: f32, n: f32, d: f32) -> f32 {
    // mulss -0x8(%rbp),%xmm0 (dest = p) ; mulss -0x8(%rbp),%xmm1 (dest = d)
    // subss %xmm1,%xmm0                 (dest = p*n)
    fsub(fmul(p, n), fmul(d, n))
}

/// `static inline float c2RayToPlane_OneDimensional(float da, float db)`
#[inline(always)]
fn c2RayToPlane_OneDimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if fmul(da, db) > 0.0 {
        // mulss -0x18(%rbp),%xmm0 with xmm0 = da
        1.0f32
    } else {
        let d = fsub(da, db);
        if d != 0.0 { fdiv(da, d) } else { 0.0 }
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
    let half_extents = c2Mulvs(c2Sub(B.max, B.min), 0.5f32);
    let center_of_b_box = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
    // subss %xmm1,%xmm0 with xmm0 = |dot(n, p0-center)|
    let d = fsub(
        tern_abs(c2Dot(n, c2Sub(p0, center_of_b_box))),
        c2Dot(abs_n, half_extents),
    );
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
    let hit0: c_int = (t0 <= 1.0f32) as c_int;
    let hit1: c_int = (t1 <= 1.0f32) as c_int;
    let hit2: c_int = (t2 <= 1.0f32) as c_int;
    let hit3: c_int = (t3 <= 1.0f32) as c_int;
    let hit: c_int = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        // cvtsi2ssl hitN -> xmm0 ; mulss %xmm1,%xmm0  (dest = (float)hitN)
        t0 = fmul(hit0 as f32, t0);
        t1 = fmul(hit1 as f32, t1);
        t2 = fmul(hit2 as f32, t2);
        t3 = fmul(hit3 as f32, t3);
        // `movss 0x20(%rbp),%xmm0` (A.t) ; `mulss -0x38(%rbp),%xmm0` (tN)
        // => the destination is A.t, not tN.
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            unsafe {
                (*out).t = fmul(A.t, t0);
                (*out).n = c2V(-1.0, 0.0);
            }
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            unsafe {
                (*out).t = fmul(A.t, t1);
                (*out).n = c2V(1.0, 0.0);
            }
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            unsafe {
                (*out).t = fmul(A.t, t2);
                (*out).n = c2V(0.0, -1.0);
            }
        } else {
            unsafe {
                (*out).t = fmul(A.t, t3);
                (*out).n = c2V(0.0, 1.0);
            }
        }
        1
    } else {
        0
    }
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
    // mulss %xmm0,%xmm1 with xmm1 = yAe.x (dest)
    if fmul(yAe.x, yAp.x) < 0.0 || tern_min(tern_abs(yAe.x), tern_abs(yAp.x)) < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if tern_abs(yAp.x) < B.r {
            if yAp.y < 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            } else {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = fsub(yAe.x, yAp.x);
            let t = fdiv(fsub(c, yAp.x), d);
            // subss %xmm2,%xmm0 (yAe.y - yAp.y) ; mulss -0x1c(%rbp),%xmm0 (* t)
            // addss %xmm1,%xmm0  => the destination is the PRODUCT, not yAp.y
            let y = fadd(fmul(fsub(yAe.y, yAp.y), t), yAp.y);
            if y <= 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            }
            if y >= yBb.y {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                    // `movss 0x20(%rbp),%xmm0` (A.t) ; `mulss -0x1c(%rbp),%xmm0`
                    (*out).t = fmul(A.t, t);
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
        // The C function falls off the end of the switch for any other value
        // (undefined behaviour); the source's dead `return 0;` is used here.
        _ => 0,
    }
}

/* -------------------------------------------------------------------------- */
/* Public entry point (include/lib.h)                                         */
/* -------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spec_ray(
    cast: *mut c2Raycast,
    mp_x: f32,
    mp_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    r_p_x: f32,
    r_p_y: f32,
) -> c_int {
    let mp = c2V(mp_x, mp_y);

    let c = c2Circle {
        p: c2V(c_p_x, c_p_y),
        r: c_r,
    };

    let mut ray = c2Ray {
        p: c2V(r_p_x, r_p_y),
        d: c2v { x: 0.0, y: 0.0 },
        t: 0.0,
    };
    ray.d = c2Norm(c2Sub(mp, ray.p));
    // subss %xmm1,%xmm0 with xmm0 = c2Dot(mp, ray.d)
    ray.t = fsub(c2Dot(mp, ray.d), c2Dot(ray.p, ray.d));

    let hit = unsafe {
        c2CastRay(
            ray,
            &c as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            cast,
        )
    };
    hit
}
