//! Rust translation of the C library in `c_src/`.
//!
//! This is a 1:1 port of `c_src/src/lib.c` (a cut-down subset of the
//! `cute_c2` 2D collision routines). Every non-static C function is exported
//! with the exact same linker symbol name and the exact same C ABI signature.
//!
//! Behavioural notes (deliberately preserved, do NOT "fix"):
//!   * `c2Maxv` / `c2Minv` use the raw `a > b ? a : b` / `a < b ? a : b`
//!     ternaries, which return the *second* operand whenever the comparison is
//!     false (e.g. for NaN operands). They are therefore NOT `f32::max` /
//!     `f32::min`, which have NaN-propagating/quieting semantics.
//!   * `c2Collided` blindly casts its first argument to `c2Circle*` regardless
//!     of the requested type, and returns 0 for any unknown `typeB`.
//!   * No guard against a degenerate (zero-length) capsule segment in
//!     `c2CircletoCapsule`: the `da / dot(n, n)` division is left as-is so that
//!     division by zero yields the same inf/NaN results as the C.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// C2_TYPE enum
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;`
///
/// An unfixed C enum whose values all fit in `int`, so it is passed as `c_int`.
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Value types
//
// Layouts must match the C exactly so that the SysV register/stack
// classification of by-value arguments and return values is identical:
//   c2v       ->  8 bytes, 1 SSE eightbyte     (xmm)
//   c2Circle  -> 12 bytes, 2 SSE eightbytes    (xmm, xmm)
//   c2AABB    -> 16 bytes, 2 SSE eightbytes    (xmm, xmm)
//   c2Capsule -> 20 bytes, > 16 bytes => MEMORY (passed on the stack)
// ---------------------------------------------------------------------------

/// `struct c2v { float x; float y; }`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `struct c2Circle { c2v p; float r; }`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `struct c2AABB { c2v min; c2v max; }`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// `struct c2Capsule { c2v a; c2v b; float r; }`
#[repr(C)]
#[derive(Copy, Clone)]
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
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

/// `c2v c2Mulvs(c2v a, float b)`
///
/// Each component multiplies with the `a` component as the `mulss` destination
/// operand, matching the C (`mulss -0xc(%rbp),%xmm0` with `a.x`/`a.y` in
/// `%xmm0`). This is observable only when both operands are NaN, where x86
/// returns `quiet(dst)`. It must be pinned with inline asm: `fmul` is
/// commutative in LLVM IR, and the release build happened to pick `a` as the
/// destination while the debug build picked `b` — a divergence that plain Rust
/// source cannot control.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let x: f32;
    let y: f32;
    unsafe {
        core::arch::asm!(
            "mulss {ax}, {b}", // a.x *= b   (dst = a.x)
            "mulss {ay}, {b}", // a.y *= b   (dst = a.y)
            ax = inout(xmm_reg) a.x => x,
            ay = inout(xmm_reg) a.y => y,
            b = in(xmm_reg) b,
            options(pure, nomem, nostack),
        );
    }
    c2v { x, y }
}

/// Portable fallback: identical for all non-NaN inputs.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x = sse_mul_src1(a.x, b);
    a.y = sse_mul_src1(a.y, b);
    a
}

/// `c2v c2Maxv(c2v a, c2v b)`
///
/// Uses the bare `>` ternary, exactly like the C macro expansion: when the
/// comparison is false (including any NaN operand) the *second* operand wins.
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

/// `c2v c2Minv(c2v a, c2v b)`
///
/// Uses the bare `<` ternary; see `c2Maxv` for the NaN behaviour.
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
// x86 SSE scalar NaN-propagation emulation
//
// `mulss`/`addss`/`subss dst, src` do NOT return a canonical NaN when an
// operand is NaN: they return `quiet(dst)` if `dst` is NaN, otherwise
// `quiet(src)` if `src` is NaN. So the *payload and sign* of the result depend
// on which operand the compiler placed in the destination register.
//
// `gcc -O0` compiles `c2Dot`'s `a.x * b.x + a.y * b.y` as
//     mulss %xmm0,%xmm1   ; xmm1 = a.x * b.x        (dst = a.x)
//     mulss %xmm2,%xmm0   ; xmm0 = b.y * a.y        (dst = b.y)
//     addss %xmm1,%xmm0   ; xmm0 = y_term + x_term  (dst = y_term)
// i.e. the *y* product is the addition's first operand, and the two products
// disagree about which side is the destination.
//
// This order cannot be pinned down from Rust source: LLVM canonicalises
// commutative `fadd`/`fmul` operands, and because LLVM's IR leaves the payload
// of a NaN-producing float op *unspecified* it will also happily fold an
// explicit `if x.is_nan() { quiet(x) } else { x + y }` back into a bare `fadd`
// (verified: doing so produced byte-identical machine code). The instructions
// are therefore emitted directly, which is the only construct LLVM may not
// reorder. Non-x86 targets fall back to the software emulation, which is
// correct for every non-NaN input and best-effort for NaN payloads.
// ---------------------------------------------------------------------------

/// `a.x * b.x + b.y * a.y`, with each SSE instruction's destination operand
/// pinned to the register the C compiler uses, so NaN payloads propagate
/// identically.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn dot_sse(a: c2v, b: c2v) -> f32 {
    let y_term: f32;
    let _x_term: f32;
    unsafe {
        core::arch::asm!(
            "mulss {x}, {bx}", // x_term = a.x * b.x   (dst = a.x)
            "mulss {y}, {ay}", // y_term = b.y * a.y   (dst = b.y)
            "addss {y}, {x}",  // y_term = y_term + x_term (dst = y_term)
            x = inout(xmm_reg) a.x => _x_term,
            y = inout(xmm_reg) b.y => y_term,
            bx = in(xmm_reg) b.x,
            ay = in(xmm_reg) a.y,
            options(pure, nomem, nostack),
        );
    }
    y_term
}

/// Portable fallback: identical for all non-NaN inputs.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn dot_sse(a: c2v, b: c2v) -> f32 {
    let x_term = sse_mul_src1(a.x, b.x);
    let y_term = sse_mul_src1(b.y, a.y);
    sse_add_src1(y_term, x_term)
}

/// True for both quiet and signalling NaN.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn is_nan_bits(x: f32) -> bool {
    (x.to_bits() & 0x7fff_ffff) > 0x7f80_0000
}

/// Set the mantissa MSB, exactly as x86 does when it quiets a NaN operand.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `src1 * src2` with `src1` as the notional destination operand.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn sse_mul_src1(src1: f32, src2: f32) -> f32 {
    if is_nan_bits(src1) {
        quiet_nan(src1)
    } else if is_nan_bits(src2) {
        quiet_nan(src2)
    } else {
        src1 * src2
    }
}

/// `src1 + src2` with `src1` as the notional destination operand.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn sse_add_src1(src1: f32, src2: f32) -> f32 {
    if is_nan_bits(src1) {
        quiet_nan(src1)
    } else if is_nan_bits(src2) {
        quiet_nan(src2)
    } else {
        src1 + src2
    }
}

/// `float c2Dot(c2v a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    dot_sse(a, b)
}

// ---------------------------------------------------------------------------
// Collision routines
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

/// `int c2Collided(const void *A, const void *B, C2_TYPE typeB)`
///
/// `A` is always reinterpreted as a `c2Circle*`, matching the C.
///
/// # Safety
/// `A` and `B` must point at properly aligned, initialised objects of the
/// types implied by `typeB`, exactly as required by the C original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(A: *const c_void, B: *const c_void, typeB: c_int) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => c2CircletoCircle(
            *(A as *const c2Circle),
            *(B as *const c2Circle),
        ),
        C2_TYPE_AABB => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),
        C2_TYPE_CAPSULE => c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule)),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Public entry point (the only symbol declared in include/lib.h)
// ---------------------------------------------------------------------------

/// `int circle_collide(float x, float y, float r)`
///
/// Tests the caller-supplied circle against three hard-coded shapes and packs
/// the three boolean results into bits 0, 1 and 2 of the return value.
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

    // The C passes addresses of these locals into c2Collided; the pointer
    // reinterpretations below are exactly the ones the C performs.
    unsafe {
        result += c2Collided(
            &circle_in as *const c2Circle as *const c_void,
            &circle as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        );

        result += c2Collided(
            &circle_in as *const c2Circle as *const c_void,
            &aabb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
        ) << 1;

        result += c2Collided(
            &circle_in as *const c2Circle as *const c_void,
            &capsule as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        ) << 2;
    }

    result
}
