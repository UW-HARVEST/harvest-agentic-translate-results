//! Rust translation of the C library in `c_src/` (a cute_c2-derived 2D raycast library).
//!
//! Every public C function is re-exported with the identical linker symbol,
//! `extern "C"` calling convention and `#[repr(C)]` layout, so the resulting
//! cdylib is ABI compatible with the original shared object.
//!
//! Behaviour is reproduced *bit-exactly*, including the original code's quirks:
//!   * the ternary based `min`/`max`/`abs` idioms, which differ from Rust's
//!     `f32::min`/`f32::max`/`f32::abs` for NaN and `-0.0` inputs;
//!   * `c2Div` performing a reciprocal followed by a multiply;
//!   * `c2Norm` of a zero vector producing NaN/inf rather than being guarded;
//!   * `c2CastRay`'s `switch` having no `default` arm, so an out-of-range
//!     `C2_TYPE` returns the caller's leftover `%eax` (reproduced with a naked
//!     dispatch shim — see `c2CastRay`).
//!
//! # Why the inline assembly
//!
//! Floating point addition and multiplication are commutative in value but *not*
//! in NaN propagation: `addss dst, src` returns `dst` quieted when `dst` is NaN,
//! and only falls back to `src` otherwise. LLVM canonicalises `fadd`/`fmul`
//! operand order independently of the source text, so a direct transcription of
//! the C expressions produces the same numbers but occasionally a NaN with the
//! opposite sign bit or a different payload than GCC's output. Since these NaNs
//! are reachable through the public `gen_ray` entry point, the commutative
//! operations are emitted explicitly with the same destination register GCC
//! chose. Non-commutative operations (`subss`, `divss`, `sqrtss`) and negation
//! (`xorps` with the sign mask) are unambiguous and use plain Rust operators.

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
// Internal types (from src/lib.c)
// ---------------------------------------------------------------------------

// The C `C2_TYPE` enum. A C enum argument is passed as `int`.
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

// ---------------------------------------------------------------------------
// Bit-exact primitives for the commutative operations.
//
// `addss(d, s)` / `mulss(d, s)` compute `d + s` / `d * s` with `d` as the SSE
// destination operand, which is what decides NaN propagation on x86.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn addss(dst: f32, src: f32) -> f32 {
    let mut d = dst;
    unsafe {
        core::arch::asm!(
            "addss {d}, {s}",
            d = inout(xmm_reg) d,
            s = in(xmm_reg) src,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    d
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn addss(dst: f32, src: f32) -> f32 {
    dst + src
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn mulss(dst: f32, src: f32) -> f32 {
    let mut d = dst;
    unsafe {
        core::arch::asm!(
            "mulss {d}, {s}",
            d = inout(xmm_reg) d,
            s = in(xmm_reg) src,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    d
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn mulss(dst: f32, src: f32) -> f32 {
    dst * src
}

// ---------------------------------------------------------------------------
// Faithful re-implementations of the C ternary idioms.
//
// These are *not* equivalent to `f32::min` / `f32::max` / `f32::abs`:
//   * `a < b ? a : b` yields `b` when either operand is NaN;
//   * `a < 0 ? -a : a` yields `-0.0` for an input of `-0.0`, and leaves a NaN's
//     sign bit untouched.
// ---------------------------------------------------------------------------

/// `a < b ? a : b`
#[inline]
fn sel_lt(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

/// `a > b ? a : b`
#[inline]
fn sel_gt(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

/// `a < 0 ? -a : a`
#[inline]
fn sel_abs(a: f32) -> f32 {
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
    // GCC: p1 = mulss(a.x, b.x); p2 = mulss(b.y, a.y); addss(p2, p1)
    let p1 = mulss(a.x, b.x);
    let p2 = mulss(b.y, a.y);
    addss(p2, p1)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    // sqrtf(c2Dot(a, a))
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    // `a.x += b.x` -- GCC emits `addss` with b as the destination operand.
    c2v {
        x: addss(b.x, a.x),
        y: addss(b.y, a.y),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    // `a.x *= b` -- destination operand is the vector component.
    c2v {
        x: mulss(a.x, b),
        y: mulss(a.y, b),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    // Reciprocal first, then multiply -- exactly as the C does.
    c2Mulvs(a, 1.0f32 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(sel_lt(a.x, b.x), sel_lt(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(sel_gt(a.x, b.x), sel_gt(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(sel_abs(a.x), sel_abs(a.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    // GCC: for each row, p1 = mulss(row.x, b.x); p2 = mulss(b.y, row.y);
    //      component = addss(p2, p1)
    let x = {
        let p1 = mulss(a.x.x, b.x);
        let p2 = mulss(b.y, a.x.y);
        addss(p2, p1)
    };
    let y = {
        let p1 = mulss(a.y.x, b.x);
        let p2 = mulss(b.y, a.y.y);
        addss(p2, p1)
    };
    c2v { x, y }
}

// ---------------------------------------------------------------------------
// Collision / raycast routines
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    let p = B.p;
    let m = c2Sub(A.p, p);
    let c = c2Dot(m, m) - mulss(B.r, B.r);
    let b = c2Dot(m, A.d);
    let disc = mulss(b, b) - c;
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

/// `static inline float c2SignedDistPointToPlane_OneDimensional(float, float, float)`
///
/// `p * n - d * n`; GCC uses `p` and `d` as the multiply destinations, so a NaN
/// `n` does not win against them and `n == -1.0` is a real multiply rather than
/// the sign flip LLVM would fold it into.
#[inline]
fn c2SignedDistPointToPlane_OneDimensional(p: f32, n: f32, d: f32) -> f32 {
    mulss(p, n) - mulss(d, n)
}

/// `static inline float c2RayToPlane_OneDimensional(float da, float db)`
#[inline]
fn c2RayToPlane_OneDimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if mulss(da, db) > 0.0 {
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
    let d = sel_abs(c2Dot(n, c2Sub(p0, center_of_b_box))) - c2Dot(abs_n, half_extents);
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
        // `(float)hitN * tN` -- the converted integer is the multiply destination.
        t0 = mulss(hit0 as f32, t0);
        t1 = mulss(hit1 as f32, t1);
        t2 = mulss(hit2 as f32, t2);
        t3 = mulss(hit3 as f32, t3);
        // `tN * A.t` -- GCC uses A.t as the multiply destination.
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            (*out).t = mulss(A.t, t0);
            (*out).n = c2V(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            (*out).t = mulss(A.t, t1);
            (*out).n = c2V(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            (*out).t = mulss(A.t, t2);
            (*out).n = c2V(0.0, -1.0);
        } else {
            (*out).t = mulss(A.t, t3);
            (*out).n = c2V(0.0, 1.0);
        }
        1
    } else {
        0
    }
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
    (d2 < mulss(A.r, A.r)) as c_int
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
    (*out).n = c2Norm(cap_n);
    (*out).t = 0.0;
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

    if mulss(yAe.x, yAp.x) < 0.0 || sel_lt(sel_abs(yAe.x), sel_abs(yAp.x)) < B.r {
        let Ca = c2Circle { p: B.a, r: B.r };
        let Cb = c2Circle { p: B.b, r: B.r };
        if sel_abs(yAp.x) < B.r {
            if yAp.y < 0.0 {
                return c2RaytoCircle(A, Ca, out);
            } else {
                return c2RaytoCircle(A, Cb, out);
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = yAe.x - yAp.x;
            let t = (c - yAp.x) / d;
            // `yAp.y + (yAe.y - yAp.y) * t` -- GCC makes the product the
            // addition's destination operand, not `yAp.y`.
            let y = addss(mulss(yAe.y - yAp.y, t), yAp.y);
            if y <= 0.0 {
                return c2RaytoCircle(A, Ca, out);
            }
            if y >= yBb.y {
                return c2RaytoCircle(A, Cb, out);
            } else {
                (*out).n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                // `t * A.t` -- GCC uses A.t as the multiply destination.
                (*out).t = mulss(A.t, t);
                return 1;
            }
        }
    }
    0
}

/// The real dispatch body. Private (no dynamic symbol) — the exported
/// `c2CastRay` is the naked shim below, which tail-jumps here for the three
/// valid `C2_TYPE` values.
unsafe extern "C" fn c2CastRay_impl(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => c2RaytoCircle(A, *(B as *const c2Circle), out),
        C2_TYPE_AABB => c2RaytoAABB(A, *(B as *const c2AABB), out),
        C2_TYPE_CAPSULE => c2RaytoCapsule(A, *(B as *const c2Capsule), out),
        // Unreachable: the shim only jumps here for 0, 1 and 2.
        _ => 0,
    }
}

/// `int c2CastRay(c2Ray A, const void *B, C2_TYPE typeB, c2Raycast *out)`
///
/// # Why this is a naked function
///
/// The C `switch` has **no `default` arm** and `c2CastRay` has **no final
/// `return`**, so for any `typeB` outside `{0, 1, 2}` control falls off the end
/// of the function. GCC (`-O0`) compiles that path to a bare `leave; ret` which
/// leaves `%eax` exactly as the caller left it, so the "return value" is the
/// caller's leftover `%eax` — verified in `objdump -d` on the C `.so`:
///
/// ```text
///   264f: cmpl $0x2,-0xc(%rbp)   ; typeB == 2 ?
///   2653: je   2701              ; -> capsule
///   2659: cmpl $0x2,-0xc(%rbp)
///   265d: ja   274b              ; unsigned > 2 -> fall through
///   ...
///   274b: leave                  ; %eax never written
///   274c: ret
/// ```
///
/// A C enum parameter is passed as a plain `int`, so an out-of-range value is a
/// real input an external caller can supply. Returning a fixed `0` here would be
/// a visible behavioural difference from the C at the ABI boundary, so the shim
/// reproduces the C exactly: dispatch for `0..=2`, otherwise `ret` without
/// touching `%eax`.
///
/// The comparison is *unsigned* (`ja`), matching the C's `cmpl $0x2` / `ja`
/// pair, so negative `typeB` values also take the fall-through path.
///
/// System V AMD64 argument placement (confirmed against the disassembly above):
/// `A` is 20 bytes and therefore MEMORY class, passed on the stack at
/// `0x10(%rbp)`; `B` is in `rdi`, `typeB` in `esi`, `out` in `rdx`. The `jmp`
/// is a tail call that leaves every register and the incoming stack argument
/// area untouched.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    core::arch::naked_asm!(
        "cmp esi, 2",
        "ja 2f",
        "jmp {imp}",
        "2:",
        "ret",
        imp = sym c2CastRay_impl,
    )
}

/// Non-x86_64 fallback: there is no portable way to "return whatever the caller
/// left in the return register", so the benign `0` stand-in is used. The C's
/// behaviour on this path is undefined and target specific.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    c2CastRay_impl(A, B, typeB, out)
}

// ---------------------------------------------------------------------------
// Public entry point (from include/lib.h)
// ---------------------------------------------------------------------------

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
    ray.t = c2Dot(mp, ray.d) - c2Dot(ray.p, ray.d);

    let c = c2Circle {
        p: c2V(c_p_x, c_p_y),
        r: c_r,
    };

    hit += c2CastRay(ray, &c as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, cast1);

    let cap = c2Capsule {
        a: c2V(cap_a_x, cap_a_y),
        b: c2V(cap_b_x, cap_b_y),
        r: cap_r,
    };

    hit += c2CastRay(
        ray,
        &cap as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        cast2,
    ) << 1;

    let bb = c2AABB {
        min: c2V(bb_min_x, bb_min_y),
        max: c2V(bb_max_x, bb_max_y),
    };

    hit += c2CastRay(ray, &bb as *const c2AABB as *const c_void, C2_TYPE_AABB, cast3) << 2;

    hit
}
