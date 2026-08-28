//! Rust translation of the C library in `c_src/`.
//!
//! Original C sources: `c_src/include/lib.h`, `c_src/src/lib.c`
//! (a trimmed-down slice of `cute_c2` by Randy Gaul — zlib / Unlicense).
//!
//! The whole public ABI of the C shared object is reproduced here, symbol for
//! symbol:
//!
//! ```text
//! c2V  c2Mulvs  c2Maxv  c2Minv  c2Clampv  c2Sub  c2Dot
//! c2CircletoCircle  c2CircletoAABB  c2CircletoCapsule
//! c2Collided  circle_collide
//! ```
//!
//! Notes on faithfulness:
//! * All arithmetic is done in `f32`, matching C `float` on x86-64 (SSE, no
//!   excess precision), so results are bit-for-bit identical.
//! * The min/max helpers reproduce C's `a > b ? a : b` ternaries verbatim
//!   rather than using `f32::max`/`f32::min`, because the two disagree on NaN
//!   and signed zero.
//! * No bugs are "fixed": e.g. `c2CircletoCapsule` still divides by
//!   `c2Dot(n, n)` without a zero check, and `c2Collided` still blindly
//!   reinterprets `A` as a `c2Circle`.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Types (from c_src/src/lib.c)
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;`
///
/// A C enum whose enumerators all fit in `int` is passed as `int`, so the
/// public entry point takes a `c_int` and compares against these constants.
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// `typedef struct c2Capsule { c2v a; c2v b; float r; } c2Capsule;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

// ---------------------------------------------------------------------------
// Bit-exact float multiply helpers
// ---------------------------------------------------------------------------
//
// IEEE-754 does not specify *which* NaN a binary operation returns, and on SSE
// the answer is "the destination operand, if it is a NaN, otherwise the source
// operand". So when both multiplicands are NaNs with different sign bits or
// payloads, the raw result bytes depend on the operand order the compiler
// happened to emit.
//
// `fmul` is commutative in LLVM IR, so plain `a * b` in Rust lets the backend
// pick either order — and it picks the opposite of GCC's choice in `c2Dot` and
// `c2Mulvs`. These helpers pin the order down to match the C shared object
// byte for byte. Everything else in this file is ordinary `f32` arithmetic.

/// `mulss dst, src` with `dst = a`: propagates `a`'s NaN in preference to `b`'s.
///
/// Matches GCC's `mulss %xmm(b),%xmm(a)` in `c2Dot`.
///
/// This has to be inline assembly: the `_mm_mul_ss` intrinsic lowers to an
/// ordinary commutative LLVM `fmul`, which the backend then re-commutes right
/// back to the order we are trying to avoid.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn mul_keep_lhs_nan(a: f32, b: f32) -> f32 {
    let mut dst = a;
    // SAFETY: `mulss` on two SSE registers has no memory or flag effects, and
    // SSE is a baseline feature of the x86_64 target.
    unsafe {
        core::arch::asm!(
            "mulss {dst}, {src}",
            dst = inout(xmm_reg) dst,
            src = in(xmm_reg) b,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    dst
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn mul_keep_lhs_nan(a: f32, b: f32) -> f32 {
    a * b
}

/// `addss dst, src` with `dst = a`: propagates `a`'s NaN in preference to `b`'s.
///
/// Matches GCC's `addss %xmm(b),%xmm(a)` in `c2Dot`.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn add_keep_lhs_nan(a: f32, b: f32) -> f32 {
    let mut dst = a;
    // SAFETY: `addss` on two SSE registers has no memory or flag effects, and
    // SSE is a baseline feature of the x86_64 target.
    unsafe {
        core::arch::asm!(
            "addss {dst}, {src}",
            dst = inout(xmm_reg) dst,
            src = in(xmm_reg) b,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    dst
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn add_keep_lhs_nan(a: f32, b: f32) -> f32 {
    a + b
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

/// ```c
/// c2v c2V(float x, float y) {
///     c2v a;
///     a.x = x;
///     a.y = y;
///     return a;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

/// ```c
/// c2v c2Mulvs(c2v a, float b) {
///     a.x *= b;
///     a.y *= b;
///     return a;
/// }
/// ```
///
/// GCC vectorises this to `mulps %xmm(a),%xmm(bbbb)` — i.e. the *broadcast
/// scalar* is the destination, so `b`'s NaN wins over `a`'s in both lanes.
/// `mul_keep_lhs_nan(b, ...)` reproduces that.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x = mul_keep_lhs_nan(b, a.x);
    a.y = mul_keep_lhs_nan(b, a.y);
    a
}

/// ```c
/// c2v c2Maxv(c2v a, c2v b) {
///     return c2V(((a.x) > (b.x) ? (a.x) : (b.x)),
///             ((a.y) > (b.y) ? (a.y) : (b.y)));
/// }
/// ```
///
/// Deliberately *not* `f32::max`: C's ternary yields `b` when the comparison is
/// unordered (NaN) and does not canonicalise `-0.0`/`+0.0`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

/// ```c
/// c2v c2Minv(c2v a, c2v b) {
///     return c2V(((a.x) < (b.x) ? (a.x) : (b.x)),
///             ((a.y) < (b.y) ? (a.y) : (b.y)));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

/// ```c
/// c2v c2Clampv(c2v a, c2v lo, c2v hi) {
///     return c2Maxv(lo, c2Minv(a, hi));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

/// ```c
/// c2v c2Sub(c2v a, c2v b) {
///     a.x -= b.x;
///     a.y -= b.y;
///     return a;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x -= b.x;
    a.y -= b.y;
    a
}

/// ```c
/// float c2Dot(c2v a, c2v b) {
///     return a.x * b.x + a.y * b.y;
/// }
/// ```
///
/// GCC emits `mulss` twice with the `a` lane as destination and then
/// `addss` with the `x` product as destination; the helpers pin that order so
/// NaN inputs yield the same bytes.
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    add_keep_lhs_nan(mul_keep_lhs_nan(a.x, b.x), mul_keep_lhs_nan(a.y, b.y))
}

// ---------------------------------------------------------------------------
// Collision routines
// ---------------------------------------------------------------------------

/// ```c
/// int c2CircletoCircle(c2Circle A, c2Circle B) {
///     c2v c = c2Sub(B.p, A.p);
///     float d2 = c2Dot(c, c);
///     float r2 = A.r + B.r;
///     r2 = r2 * r2;
///     return d2 < r2;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

/// ```c
/// int c2CircletoAABB(c2Circle A, c2AABB B) {
///     c2v L = c2Clampv(A.p, B.min, B.max);
///     c2v ab = c2Sub(A.p, L);
///     float d2 = c2Dot(ab, ab);
///     float r2 = A.r * A.r;
///     return d2 < r2;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

/// ```c
/// int c2CircletoCapsule(c2Circle A, c2Capsule B) {
///     c2v n = c2Sub(B.b, B.a);
///     c2v ap = c2Sub(A.p, B.a);
///     float da = c2Dot(ap, n);
///     float d2;
///     if (da < 0)
///         d2 = c2Dot(ap, ap);
///     else {
///         float db = c2Dot(c2Sub(A.p, B.b), n);
///         if (db < 0) {
///             c2v e = c2Sub(ap, c2Mulvs(n, (da / c2Dot(n, n))));
///             d2 = c2Dot(e, e);
///         } else {
///             c2v bp = c2Sub(A.p, B.b);
///             d2 = c2Dot(bp, bp);
///         }
///     }
///     float r = A.r + B.r;
///     return d2 < r * r;
/// }
/// ```
///
/// The `da / c2Dot(n, n)` division has no zero guard in the C original (a
/// degenerate capsule with `a == b` produces inf/NaN); that is preserved.
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2: f32;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = A.r + B.r;
    (d2 < r * r) as c_int
}

/// ```c
/// int c2Collided(const void *A, const void *B, C2_TYPE typeB) {
///     switch (typeB) {
///         case C2_TYPE_CIRCLE:  return c2CircletoCircle(*(c2Circle *)A, *(c2Circle *)B);
///         case C2_TYPE_AABB:    return c2CircletoAABB(*(c2Circle *)A, *(c2AABB *)B);
///         case C2_TYPE_CAPSULE: return c2CircletoCapsule(*(c2Circle *)A, *(c2Capsule *)B);
///         default:              return 0;
///     }
/// }
/// ```
///
/// `A` is always reinterpreted as a `c2Circle` regardless of `typeB`, exactly
/// as in the C original.
///
/// # Safety
///
/// `A` must point to a readable `c2Circle` and `B` to a readable object of the
/// shape selected by `typeB`, for any `typeB` in `0..=2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(A: *const c_void, B: *const c_void, typeB: c_int) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => c2CircletoCircle(
            unsafe { (A as *const c2Circle).read_unaligned() },
            unsafe { (B as *const c2Circle).read_unaligned() },
        ),
        C2_TYPE_AABB => c2CircletoAABB(
            unsafe { (A as *const c2Circle).read_unaligned() },
            unsafe { (B as *const c2AABB).read_unaligned() },
        ),
        C2_TYPE_CAPSULE => c2CircletoCapsule(
            unsafe { (A as *const c2Circle).read_unaligned() },
            unsafe { (B as *const c2Capsule).read_unaligned() },
        ),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Public entry point (c_src/include/lib.h)
// ---------------------------------------------------------------------------

/// ```c
/// int circle_collide(float x, float y, float r);
/// ```
///
/// Tests the circle `((x, y), r)` against three hard-coded shapes and packs the
/// three boolean results into bits 0, 1 and 2 of the return value.
#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let mut circle_in = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    circle_in.p = c2V(x, y);
    circle_in.r = r;

    let mut circle = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    circle.p = c2V(-70.0f32, 0.0);
    circle.r = 20.0f32;

    let mut aabb = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 0.0, y: 0.0 },
    };
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);

    let mut capsule = c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    capsule.a = c2V(-40.0f32, 40.0f32);
    capsule.b = c2V(-20.0f32, 100.0f32);
    capsule.r = 10.0f32;

    let circle_in_p: *const c_void = (&raw const circle_in).cast();
    let circle_p: *const c_void = (&raw const circle).cast();
    let aabb_p: *const c_void = (&raw const aabb).cast();
    let capsule_p: *const c_void = (&raw const capsule).cast();

    result += unsafe { c2Collided(circle_in_p, circle_p, C2_TYPE_CIRCLE) };

    result += unsafe { c2Collided(circle_in_p, aabb_p, C2_TYPE_AABB) } << 1;

    result += unsafe { c2Collided(circle_in_p, capsule_p, C2_TYPE_CAPSULE) } << 2;

    result
}
