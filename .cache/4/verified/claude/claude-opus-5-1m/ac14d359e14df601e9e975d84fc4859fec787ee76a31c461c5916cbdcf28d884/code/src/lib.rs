//! Rust translation of the C library in `c_src/`.
//!
//! This is a faithful, ABI-compatible port of `c_src/src/lib.c` (a trimmed-down
//! version of the `cute_c2` 2D collision routines). Every public symbol exported
//! by the C shared library is re-exported here with an identical signature so
//! that the resulting `cdylib` is a drop-in replacement.
//!
//! Behavioural notes (deliberately preserving C semantics, bugs included):
//! * `c2Maxv` / `c2Minv` use raw `>` / `<` comparisons rather than
//!   `f32::max` / `f32::min`, so NaN propagation matches C exactly (a NaN
//!   comparison is false, hence the second operand is selected).
//! * `c2AABBtoAABB` reproduces the C `int` bit-OR of four comparison results.
//! * `collided` returns `0` for any unrecognised `C2_TYPE` discriminant, in the
//!   same check order as the original nested `switch` statements.

#![allow(non_snake_case)]

use std::ffi::c_int;
use std::os::raw::c_void;

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB } C2_TYPE;`
///
/// A C enum with these enumerators is `int`-sized, so it is represented as
/// [`c_int`] in the FFI signatures below.
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

/// `c2v c2V(float x, float y)`
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

/// `c2v c2Maxv(c2v a, c2v b)`
///
/// Uses the C ternary form `a > b ? a : b`; when either operand is NaN the
/// comparison is false and `b` is returned, exactly as in C.
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

/// `c2v c2Minv(c2v a, c2v b)`
///
/// Uses the C ternary form `a < b ? a : b`; NaN operands select `b`.
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

// ---------------------------------------------------------------------------
// x86 SSE NaN-propagation helpers
// ---------------------------------------------------------------------------
//
// `mulss`/`addss` propagate the *destination* operand when it is a NaN, and
// only fall back to the source operand otherwise (Intel SDM, "Operation" table
// for MULSS/ADDSS). The C reference build (`c_src/CMakeLists.txt` sets no
// `CMAKE_BUILD_TYPE`, i.e. `-O0`) picks a specific pair of destinations for
// `c2Dot`, so the observable NaN payload of its return value depends on that
// choice. These helpers make the Rust translation reproduce it explicitly
// instead of relying on whatever operand order LLVM happens to select, so the
// behaviour is identical in debug and release builds.
//
// For operands that are not NaN the helpers are a plain `*` / `+`: both
// operations are commutative for non-NaN inputs (including the signed zeros
// and the invalid-operation cases `0 * Inf` and `Inf + -Inf`, which yield the
// same default QNaN either way), so no other behaviour is affected.

/// Quiet a NaN exactly as x86 does when propagating it: set the significand's
/// most-significant bit and preserve the sign and payload. Already-quiet NaNs
/// pass through unchanged.
#[inline]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `mulss dest, src`
#[inline]
fn mulss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet_nan(dest)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dest * src
    }
}

/// `addss dest, src`
#[inline]
fn addss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet_nan(dest)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dest + src
    }
}

/// `float c2Dot(c2v a, c2v b)`
///
/// `return a.x * b.x + a.y * b.y;`
///
/// The reference (`-O0`) build emits:
///
/// ```text
///   mulss %xmm0,%xmm1   ; xmm1 = a.x * b.x      (destination = a.x)
///   mulss %xmm2,%xmm0   ; xmm0 = b.y * a.y      (destination = b.y)
///   addss %xmm1,%xmm0   ; xmm0 = y_prod + x_prod (destination = y_prod)
/// ```
///
/// so the NaN-propagation priority is `a.x` before `b.x` for the x product,
/// `b.y` before `a.y` for the y product, and the y product before the x
/// product for the sum. Numerically this is exactly `a.x*b.x + a.y*b.y`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    let x_prod = mulss(a.x, b.x);
    let y_prod = mulss(b.y, a.y);
    addss(y_prod, x_prod)
}

// ---------------------------------------------------------------------------
// Boolean collision routines
// ---------------------------------------------------------------------------

/// `int c2CircletoCircle(c2Circle A, c2Circle B)`
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let r2 = A.r + B.r;
    let r2 = r2 * r2;
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

/// `int c2AABBtoAABB(c2AABB A, c2AABB B)`
///
/// Mirrors the C implementation's four `int` flags combined with a bitwise OR
/// and then logically negated.
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

// ---------------------------------------------------------------------------
// Public dispatch entry point
// ---------------------------------------------------------------------------

/// `int collided(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB)`
///
/// Dispatches on the pair of shape tags, reading the shape data out of the
/// untyped pointers just like the C casts do. Unknown tags yield `0`.
///
/// # Safety
/// `A` and `B` must point to fully initialised objects of the type indicated by
/// `typeA` / `typeB` respectively, matching the C contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(
                unsafe { (A as *const c2Circle).read_unaligned() },
                unsafe { (B as *const c2Circle).read_unaligned() },
            ),
            C2_TYPE_AABB => c2CircletoAABB(
                unsafe { (A as *const c2Circle).read_unaligned() },
                unsafe { (B as *const c2AABB).read_unaligned() },
            ),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(
                unsafe { (B as *const c2Circle).read_unaligned() },
                unsafe { (A as *const c2AABB).read_unaligned() },
            ),
            C2_TYPE_AABB => c2AABBtoAABB(
                unsafe { (A as *const c2AABB).read_unaligned() },
                unsafe { (B as *const c2AABB).read_unaligned() },
            ),
            _ => 0,
        },
        _ => 0,
    }
}
