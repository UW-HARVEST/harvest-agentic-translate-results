//! Rust translation of `c_src/src/lib.c` (a trimmed-down slice of cute_c2).
//!
//! The public entry point declared in `c_src/include/lib.h` is `circle_collide`.
//! The header contains no namespacing macros, so the linker symbols keep their
//! source-level names.
//!
//! All arithmetic is single precision (`f32`) and every comparison is
//! reproduced literally (including the NaN-propagation behaviour of the C
//! ternary based min/max helpers) so that results are bit-identical to the C.

// C identifiers are kept verbatim so the exported symbols match the C library.
#![allow(non_camel_case_types, non_snake_case)]

use std::ffi::{c_int, c_void};

// C2_TYPE enumerators from lib.c
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

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

// ---------------------------------------------------------------------------
// Scalar arithmetic with pinned x86 SSE operand semantics
// ---------------------------------------------------------------------------
//
// For `MULSS`/`ADDSS`/`SUBSS` the *destination* operand's NaN payload takes
// precedence over the source operand's, and a signalling NaN is quieted on the
// way out. Which side of a Rust `+`/`-`/`*` becomes the destination register is
// entirely up to LLVM and is not guaranteed by the language, whereas the C we
// are matching has a fixed instruction sequence:
//
//   c2Sub    subss %xmm1,%xmm0          dst = a.<c>, src = b.<c>
//   c2Mulvs  mulss -0xc(%rbp),%xmm0     dst = a.<c>, src = b
//   c2Dot    mulss %xmm0,%xmm1          dst = a.x,   src = b.x
//            mulss %xmm2,%xmm0          dst = b.y,   src = a.y   <- operands swapped
//            addss %xmm1,%xmm0          dst = y product, src = x product
//
// Spelling the NaN selection out here makes the result independent of how the
// Rust code is lowered: `c2Dot(c2v { x: 0.0, y: NAN }, c2v { x: INFINITY, y: 0.0 })`
// must yield the payload of the y product, not the `0.0 * INFINITY` indefinite.

/// Sets the quiet bit, matching the SNaN -> QNaN conversion the FPU performs.
#[inline(always)]
fn quiet_nan(v: f32) -> f32 {
    f32::from_bits(v.to_bits() | 0x0040_0000)
}

/// `Some(result)` when an SSE scalar op would short-circuit on a NaN operand.
#[inline(always)]
fn sse_nan_result(dst: f32, src: f32) -> Option<f32> {
    if dst.is_nan() {
        Some(quiet_nan(dst))
    } else if src.is_nan() {
        Some(quiet_nan(src))
    } else {
        None
    }
}

/// `mulss dst, src`
#[inline(always)]
fn mulss(dst: f32, src: f32) -> f32 {
    match sse_nan_result(dst, src) {
        Some(nan) => nan,
        None => dst * src,
    }
}

/// `addss dst, src`
#[inline(always)]
fn addss(dst: f32, src: f32) -> f32 {
    match sse_nan_result(dst, src) {
        Some(nan) => nan,
        None => dst + src,
    }
}

/// `subss dst, src`
#[inline(always)]
fn subss(dst: f32, src: f32) -> f32 {
    match sse_nan_result(dst, src) {
        Some(nan) => nan,
        None => dst - src,
    }
}

/// `divss dst, src`
#[inline(always)]
fn divss(dst: f32, src: f32) -> f32 {
    match sse_nan_result(dst, src) {
        Some(nan) => nan,
        None => dst / src,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    c2v {
        x: mulss(a.x, b),
        y: mulss(a.y, b),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    // `(a.x) > (b.x) ? (a.x) : (b.x)` -- not f32::max: a NaN comparison is
    // false, so `b` wins, which differs from `f32::max`.
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: subss(a.x, b.x),
        y: subss(a.y, b.y),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    let x_product = mulss(a.x, b.x);
    // The C's `a.y * b.y` is emitted with `b.y` as the destination register.
    let y_product = mulss(b.y, a.y);
    // ...and the add uses the y product as its destination.
    addss(y_product, x_product)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    // `addss %xmm1,%xmm0` with xmm0 = B.r: the C emits `A.r + B.r` with `B.r`
    // as the destination, so a NaN in `B.r` takes precedence.
    let mut r2 = addss(B.r, A.r);
    r2 = mulss(r2, r2);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = mulss(A.r, A.r);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2;
    // `comiss` + `jbe`: an unordered compare falls through to the `else`, which
    // is what `da < 0.0` does in Rust for a NaN as well.
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, divss(da, c2Dot(n, n))));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    // Same swapped `addss` destination as in `c2CircletoCircle`.
    let r = addss(B.r, A.r);
    (d2 < mulss(r, r)) as c_int
}

/// `A` is unconditionally reinterpreted as a `c2Circle`, exactly as the C does.
///
/// # Safety
///
/// `A` must point to at least `size_of::<c2Circle>()` readable bytes. `B` must
/// point to a readable object of the layout selected by `typeB`: `c2Circle`,
/// `c2AABB` or `c2Capsule`. Any other `typeB` value dereferences neither
/// pointer beyond `A`. This mirrors the C, which performs the same unchecked
/// casts.
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

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let circle_in = c2Circle {
        p: c2V(x, y),
        r,
    };

    let circle = c2Circle {
        p: c2V(-70.0f32, 0.0),
        r: 20.0f32,
    };

    let aabb = c2AABB {
        min: c2V(-40.0f32, -40.0f32),
        max: c2V(-15.0f32, -15.0f32),
    };

    let capsule = c2Capsule {
        a: c2V(-40.0f32, 40.0f32),
        b: c2V(-20.0f32, 100.0f32),
        r: 10.0f32,
    };

    let a = (&circle_in as *const c2Circle).cast::<c_void>();

    result += unsafe {
        c2Collided(
            a,
            (&circle as *const c2Circle).cast::<c_void>(),
            C2_TYPE_CIRCLE,
        )
    };

    result += unsafe {
        c2Collided(a, (&aabb as *const c2AABB).cast::<c_void>(), C2_TYPE_AABB)
    } << 1;

    result += unsafe {
        c2Collided(
            a,
            (&capsule as *const c2Capsule).cast::<c_void>(),
            C2_TYPE_CAPSULE,
        )
    } << 2;

    result
}
