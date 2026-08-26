//! Rust translation of the C library in `c_src/` (tinyc2-style 2D collision
//! routines).
//!
//! The C build globs all of `c_src/` into a single shared library; every public
//! symbol exported by that `.so` is reproduced here with an identical name,
//! signature and ABI.
//!
//! Exported symbols (from `nm -D` on the C `.so`):
//!   c2V, c2Mulvs, c2Maxv, c2Minv, c2Clampv, c2Sub, c2Dot,
//!   c2CircletoCircle, c2CircletoAABB, c2CircletoCapsule,
//!   c2Collided, circle_collide
//!
//! Behaviour is reproduced exactly, including the original C code's quirks
//! (e.g. `c2Collided` always reinterpreting `A` as a `c2Circle` regardless of
//! any type tag, and the `?:`-based min/max NaN semantics).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_int;
use std::os::raw::c_void;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;`
///
/// A C enum whose values all fit in `int` is passed as `int` in the SysV ABI,
/// so the FFI boundary uses `c_int` and these constants are compared against
/// it.
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// `typedef struct c2Capsule { c2v a; c2v b; float r; } c2Capsule;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

/// `c2v c2V(float x, float y)`
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v::default();
    a.x = x;
    a.y = y;
    a
}

/// `c2v c2Mulvs(c2v a, float b)`
///
/// The C code compiles `a.x *= b` to `mulss <b>, %xmm(a.x)`, i.e. the vector
/// component is the *destination* operand. Per the SSE NaN rules the
/// destination (first) operand wins when both operands are NaN, so a NaN
/// component must keep its own payload rather than `b`'s. LLVM is free to
/// commute `fmul` and in practice emits the operands the other way round, so
/// the both-NaN case is pinned explicitly via [`nan_ordered`]. Ordinary values
/// take the plain arithmetic path and are unaffected.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x = nan_ordered(a.x, b, false, |l, r| l * r);
    a.y = nan_ordered(a.y, b, false, |l, r| l * r);
    a
}

/// `c2v c2Maxv(c2v a, c2v b)`
///
/// The C code expands to `(a.x) > (b.x) ? (a.x) : (b.x)`, which selects `b`
/// whenever the comparison is false (including when either operand is NaN).
/// `f32::max` would instead ignore NaN, so the ternary is reproduced literally.
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

/// `c2v c2Minv(c2v a, c2v b)`
///
/// As with [`c2Maxv`], the `?:` selection semantics (including NaN behaviour)
/// are reproduced exactly rather than using `f32::min`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

/// `c2v c2Clampv(c2v a, c2v lo, c2v hi)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

/// `c2v c2Sub(c2v a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x -= b.x;
    a.y -= b.y;
    a
}

/// Quiets a NaN the way x86-64 SSE does: set the quiet bit, keep the sign and
/// the rest of the payload.
#[inline]
fn quiet_nan(v: f32) -> f32 {
    f32::from_bits(v.to_bits() | 0x0040_0000)
}

/// A binary float op that reproduces the exact NaN payload the C build
/// propagates.
///
/// `mulss`/`addss` return the destination operand (quieted) when it is NaN, and
/// otherwise the source operand (quieted) — so the payload only depends on
/// operand ordering when *both* inputs are NaN. `keep_rhs` says which side the
/// C code's destination register held in that case, as measured against the
/// compiled C library. Every non-NaN input takes the plain arithmetic path, so
/// this is bit-identical to `lhs op rhs` for ordinary values.
#[inline]
fn nan_ordered(lhs: f32, rhs: f32, keep_rhs: bool, op: impl Fn(f32, f32) -> f32) -> f32 {
    if lhs.is_nan() && rhs.is_nan() {
        return quiet_nan(if keep_rhs { rhs } else { lhs });
    }
    op(lhs, rhs)
}

/// `float c2Dot(c2v a, c2v b)`
///
/// The expression is `a.x * b.x + a.y * b.y`. For the products, the C code
/// keeps the left payload for the `x` term and the right payload for the `y`
/// term; the sum keeps the right (second product's) payload. Ordinary values
/// are unaffected by this bookkeeping.
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    let px = nan_ordered(a.x, b.x, false, |l, r| l * r);
    let py = nan_ordered(a.y, b.y, true, |l, r| l * r);
    nan_ordered(px, py, true, |l, r| l + r)
}

// ---------------------------------------------------------------------------
// Collision tests
// ---------------------------------------------------------------------------

/// `int c2CircletoCircle(c2Circle A, c2Circle B)`
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

/// `int c2CircletoAABB(c2Circle A, c2AABB B)`
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

/// `int c2CircletoCapsule(c2Circle A, c2Capsule B)`
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2;
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

/// `int c2Collided(const void *A, const void *B, C2_TYPE typeB)`
///
/// Note that, exactly as in the C original, `A` is always dereferenced as a
/// `c2Circle` — only `B` is dispatched on `typeB`. Unknown tags return 0.
///
/// # Safety
///
/// `A` and `B` must point to appropriately-sized, initialised objects
/// (`c2Circle` for `A`; the type selected by `typeB` for `B`), just as the C
/// function requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(A: *const c_void, B: *const c_void, typeB: c_int) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => c2CircletoCircle(
            unsafe { *(A as *const c2Circle) },
            unsafe { *(B as *const c2Circle) },
        ),
        C2_TYPE_AABB => c2CircletoAABB(
            unsafe { *(A as *const c2Circle) },
            unsafe { *(B as *const c2AABB) },
        ),
        C2_TYPE_CAPSULE => c2CircletoCapsule(
            unsafe { *(A as *const c2Circle) },
            unsafe { *(B as *const c2Capsule) },
        ),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// `int circle_collide(float x, float y, float r)`
///
/// Tests the given circle against a fixed circle, AABB and capsule, packing
/// the three results into bits 0, 1 and 2 of the return value.
#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let mut circle_in = c2Circle::default();
    circle_in.p = c2V(x, y);
    circle_in.r = r;

    let mut circle = c2Circle::default();
    circle.p = c2V(-70.0f32, 0.0);
    circle.r = 20.0f32;

    let mut aabb = c2AABB::default();
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);

    let mut capsule = c2Capsule::default();
    capsule.a = c2V(-40.0f32, 40.0f32);
    capsule.b = c2V(-20.0f32, 100.0f32);
    capsule.r = 10.0f32;

    let circle_in_p = &circle_in as *const c2Circle as *const c_void;

    result += unsafe {
        c2Collided(
            circle_in_p,
            &circle as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        )
    };

    result += unsafe {
        c2Collided(
            circle_in_p,
            &aabb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
        )
    } << 1;

    result += unsafe {
        c2Collided(
            circle_in_p,
            &capsule as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        )
    } << 2;

    result
}
