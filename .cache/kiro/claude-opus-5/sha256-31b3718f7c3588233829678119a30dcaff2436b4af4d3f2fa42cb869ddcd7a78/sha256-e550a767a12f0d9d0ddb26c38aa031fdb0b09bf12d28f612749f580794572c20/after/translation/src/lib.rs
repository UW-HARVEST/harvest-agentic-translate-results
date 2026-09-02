//! Rust translation of the C library in `c_src/`.
//!
//! Every public symbol exported by the C shared object is reproduced here with
//! the same linker name, the same C ABI signature, and the same observable
//! behaviour -- including the quirks and bugs of the original.
//!
//! # Faithfulness notes
//!
//! * `min`/`max`/`abs` are written in the C as ternary expressions, not libm
//!   calls. Those have different NaN and negative-zero behaviour than
//!   `f32::min`, `f32::max` and `f32::abs`, so they are reproduced literally.
//! * `c2Div` multiplies by the reciprocal (`a * (1/b)`) rather than dividing.
//!   That is not the same value bit-for-bit, and is preserved.
//! * `c2CastRay` falls off the end of a non-void function for unknown tags; see
//!   the comment there.
//! * Float addition and multiplication are commutative bit-for-bit for every
//!   input *except* a pair of NaNs: x86 `addss`/`mulss` return the destination
//!   operand's payload. LLVM canonicalises commutative operands however it
//!   likes, so source-level operand order cannot control this. The `fadd_*` /
//!   `fmul_*` helpers below pin the destination register with inline assembly,
//!   chosen per call site to match the reference C build. See `float_ops`.

#![allow(non_snake_case)]

use std::ffi::{c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// Public types (from include/lib.h)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Internal types (from src/lib.c)
// ---------------------------------------------------------------------------

// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;
// All enumerators are non-negative, so the underlying type is `unsigned int`.
const C2_TYPE_CIRCLE: c_uint = 0;
const C2_TYPE_AABB: c_uint = 1;
const C2_TYPE_CAPSULE: c_uint = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
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
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

// ---------------------------------------------------------------------------
// Scalar float primitives with a pinned destination operand.
// ---------------------------------------------------------------------------

/// Scalar single-precision arithmetic with an explicitly chosen x86
/// destination operand.
///
/// For every input except a pair of NaNs these are plain IEEE-754 operations
/// and the `_l` / `_r` variants are indistinguishable. When both operands are
/// NaN, `addss`/`mulss` propagate the *destination* operand's payload, so the
/// choice becomes observable. Picking it explicitly is what makes the output
/// bit-identical to the reference C build rather than merely numerically equal.
mod float_ops {
    /// `dst = dst OP src`, i.e. the result carries `dst`'s NaN payload.
    macro_rules! sse_binop {
        ($name:ident, $mnemonic:literal) => {
            #[cfg(target_arch = "x86_64")]
            #[inline(always)]
            fn $name(dst: f32, src: f32) -> f32 {
                let mut d = dst;
                // SAFETY: a single SSE arithmetic instruction on two scalar
                // f32 values in XMM registers. It touches no memory, no
                // flags, and no stack; `pure` is valid because the result
                // depends only on the inputs.
                unsafe {
                    core::arch::asm!(
                        concat!($mnemonic, " {d}, {s}"),
                        d = inout(xmm_reg) d,
                        s = in(xmm_reg) src,
                        options(pure, nomem, nostack, preserves_flags),
                    );
                }
                d
            }
        };
    }

    sse_binop!(addss, "addss");
    sse_binop!(subss, "subss");
    sse_binop!(mulss, "mulss");
    sse_binop!(divss, "divss");

    /// `a + b`, taking `b` as the destination (so `b`'s NaN payload wins).
    #[inline(always)]
    pub fn fadd_r(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            addss(b, a)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a + b
        }
    }

    /// `a * b`, taking `a` as the destination.
    #[inline(always)]
    pub fn fmul_l(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            mulss(a, b)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a * b
        }
    }

    /// `a * b`, taking `b` as the destination.
    #[inline(always)]
    pub fn fmul_r(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            mulss(b, a)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a * b
        }
    }

    /// `a - b`. Subtraction is not commutative, so the destination is always
    /// the left operand; there is no choice to make.
    #[inline(always)]
    pub fn fsub(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            subss(a, b)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a - b
        }
    }

    /// `a / b`. As with subtraction there is no operand choice.
    #[inline(always)]
    pub fn fdiv(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            divss(a, b)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a / b
        }
    }
}

use float_ops::{fadd_r, fdiv, fmul_l, fmul_r, fsub};

// ---------------------------------------------------------------------------
// Ternary helpers: literal translations of the C macro expansions.
// ---------------------------------------------------------------------------

/// `((a) < (b) ? (a) : (b))`
#[inline]
fn ter_min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

/// `((a) > (b) ? (a) : (b))`
#[inline]
fn ter_max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

/// `((a) < 0 ? -(a) : (a))`
#[inline]
fn ter_abs(a: f32) -> f32 {
    if a < 0.0 {
        -a
    } else {
        a
    }
}

// ---------------------------------------------------------------------------
// Vector math
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // a.x * b.x + a.y * b.y
    let p1 = fmul_l(a.x, b.x);
    let p2 = fmul_r(a.y, b.y);
    fadd_r(p1, p2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    // sqrtf(c2Dot(a, a))
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x = fadd_r(a.x, b.x);
    a.y = fadd_r(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x = fsub(a.x, b.x);
    a.y = fsub(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x = fmul_l(a.x, b);
    a.y = fmul_l(a.y, b);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    // c2Mulvs(a, 1.0f / b) -- reciprocal then multiply, not a division.
    c2Mulvs(a, fdiv(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(ter_min(a.x, b.x), ter_min(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(ter_max(a.x, b.x), ter_max(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(ter_abs(a.x), ter_abs(a.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    // c.x = a.x.x * b.x + a.x.y * b.y;
    // c.y = a.y.x * b.x + a.y.y * b.y;
    let x1 = fmul_l(a.x.x, b.x);
    let x2 = fmul_r(a.x.y, b.y);
    let y1 = fmul_l(a.y.x, b.x);
    let y2 = fmul_r(a.y.y, b.y);
    c2v {
        x: fadd_r(x1, x2),
        y: fadd_r(y1, y2),
    }
}

// ---------------------------------------------------------------------------
// Overlap / containment tests
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
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
    (d2 < fmul_l(A.r, A.r)) as c_int
}

// ---------------------------------------------------------------------------
// Raycasts
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    let p = B.p;
    let m = c2Sub(A.p, p);
    let c = fsub(c2Dot(m, m), fmul_l(B.r, B.r));
    let b = c2Dot(m, A.d);
    let disc = fsub(fmul_l(b, b), c);
    if disc < 0.0 {
        return 0;
    }
    let t = fsub(-b, disc.sqrt());
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

/// `static inline float c2SignedDistPointToPlane_OneDimensional(float, float, float)`
#[inline]
fn c2SignedDistPointToPlane_OneDimensional(p: f32, n: f32, d: f32) -> f32 {
    // p * n - d * n
    fsub(fmul_l(p, n), fmul_l(d, n))
}

/// `static inline float c2RayToPlane_OneDimensional(float da, float db)`
#[inline]
fn c2RayToPlane_OneDimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if fmul_l(da, db) > 0.0 {
        1.0f32
    } else {
        let d = fsub(da, db);
        if d != 0.0 {
            fdiv(da, d)
        } else {
            0.0
        }
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
    // The C spells the |dot| out as a ternary three times over; the value is
    // the same each time.
    let d = fsub(
        ter_abs(c2Dot(n, c2Sub(p0, center_of_b_box))),
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
    let hit0 = (t0 <= 1.0f32) as c_int;
    let hit1 = (t1 <= 1.0f32) as c_int;
    let hit2 = (t2 <= 1.0f32) as c_int;
    let hit3 = (t3 <= 1.0f32) as c_int;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = fmul_l(hit0 as f32, t0);
        t1 = fmul_l(hit1 as f32, t1);
        t2 = fmul_l(hit2 as f32, t2);
        t3 = fmul_l(hit3 as f32, t3);
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            unsafe {
                (*out).t = fmul_r(t0, A.t);
                (*out).n = c2V(-1.0, 0.0);
            }
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            unsafe {
                (*out).t = fmul_r(t1, A.t);
                (*out).n = c2V(1.0, 0.0);
            }
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            unsafe {
                (*out).t = fmul_r(t2, A.t);
                (*out).n = c2V(0.0, -1.0);
            }
        } else {
            unsafe {
                (*out).t = fmul_r(t3, A.t);
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
    if fmul_l(yAe.x, yAp.x) < 0.0 || ter_min(ter_abs(yAe.x), ter_abs(yAp.x)) < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if ter_abs(yAp.x) < B.r {
            if yAp.y < 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            } else {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = fsub(yAe.x, yAp.x);
            let t = fdiv(fsub(c, yAp.x), d);
            let y = fadd_r(yAp.y, fmul_l(fsub(yAe.y, yAp.y), t));
            if y <= 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            }
            if y >= yBb.y {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                    (*out).t = fmul_r(t, A.t);
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
    typeB: c_uint,
    out: *mut c2Raycast,
) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => unsafe { c2RaytoCircle(A, *(B as *const c2Circle), out) },
        C2_TYPE_AABB => unsafe { c2RaytoAABB(A, *(B as *const c2AABB), out) },
        C2_TYPE_CAPSULE => unsafe { c2RaytoCapsule(A, *(B as *const c2Capsule), out) },
        // The C `switch` has no `default` label and no trailing `return`, so
        // control flows off the end of a non-void function for unknown tags --
        // undefined behaviour. On x86-64 SysV the compiled C leaves the first
        // integer argument register in `%eax` and returns that; since the
        // 20-byte `c2Ray` is passed in memory, that register holds `B`.
        // Reproduced so the observable return value matches.
        _ => B as usize as c_int,
    }
}

// ---------------------------------------------------------------------------
// Public entry point (declared in include/lib.h)
// ---------------------------------------------------------------------------

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
    ray.t = fsub(c2Dot(mp, ray.d), c2Dot(ray.p, ray.d));

    unsafe {
        c2CastRay(
            ray,
            &c as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            cast,
        )
    }
}
