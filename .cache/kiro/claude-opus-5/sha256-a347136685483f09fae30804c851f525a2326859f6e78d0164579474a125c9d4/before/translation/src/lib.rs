//! Rust translation of `c_src/src/lib.c` (public header: `c_src/include/lib.h`).
//!
//! Public ABI surface of the C shared library (per `nm -D --defined-only`):
//!   * `to_barycentric`
//!
//! `lm_v2` / `lm_sub2` / `lm_dot2` are `static` in the C source, so they are not
//! exported; they are reproduced here as private helpers with identical
//! semantics.
//!
//! ## Bit-exactness notes
//!
//! The C library compiles to plain scalar SSE (`subss`/`mulss`/`addss`/`divss`).
//! Two codegen details are observable in the results and have to be mirrored for
//! byte-identical output across the whole input domain (including infinities and
//! NaNs):
//!
//! 1. **Scalar, not packed.** Left to itself LLVM fuses the `x`/`y` lanes into
//!    packed `subps`/`mulps`/`addps`, which changes which operand a NaN is taken
//!    from.
//! 2. **Operand order of the commutative ops.** An x86 SSE binary op returns the
//!    *first* (destination) operand when both operands are NaN, so operand order
//!    is observable through the NaN sign bit and payload even though it is
//!    numerically irrelevant otherwise. While inlining `lm_dot2`, GCC swapped
//!    the operands of several multiplies and of one addition.
//!
//! Ordinary Rust source order does not survive LLVM's canonicalisation of
//! commutative operands, so on x86-64 each arithmetic step is emitted as a
//! single explicit instruction via `asm!`, with the destination register chosen
//! to match the instruction GCC actually emitted (annotated at each call site in
//! [`to_barycentric`]). Only commutative operands are reordered relative to the
//! C source, so every non-NaN result is unchanged; subtraction and division
//! appear in source order.
//!
//! On non-x86-64 targets the same operations fall back to plain Rust float
//! arithmetic, which is identical except for NaN payload provenance.

#![allow(non_camel_case_types)]

/// ```c
/// typedef struct lm_vec2 {
///     float x, y;
/// } lm_vec2;
/// ```
///
/// Two consecutive `float`s: under the SysV x86-64 ABI this classifies as a
/// single SSE eightbyte, so it is passed and returned packed in one XMM
/// register. `repr(C)` makes rustc apply the same platform classification.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

/// Scalar single-precision primitives.
///
/// Each function performs exactly one IEEE-754 binary32 operation with `a` as
/// the destination operand, which is what decides NaN propagation on x86.
mod ops {
    /// `a * b` (`mulss a, b`)
    #[inline]
    pub fn mul(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            let mut r = a;
            // SAFETY: one SSE arithmetic instruction on two scalar float
            // registers; no memory access, no flag or stack effects.
            unsafe {
                core::arch::asm!(
                    "mulss {r}, {b}",
                    r = inout(xmm_reg) r,
                    b = in(xmm_reg) b,
                    options(pure, nomem, nostack, preserves_flags),
                );
            }
            r
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a * b
        }
    }

    /// `a + b` (`addss a, b`)
    #[inline]
    pub fn add(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            let mut r = a;
            // SAFETY: see `mul`.
            unsafe {
                core::arch::asm!(
                    "addss {r}, {b}",
                    r = inout(xmm_reg) r,
                    b = in(xmm_reg) b,
                    options(pure, nomem, nostack, preserves_flags),
                );
            }
            r
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a + b
        }
    }

    /// `a - b` (`subss a, b`)
    #[inline]
    pub fn sub(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            let mut r = a;
            // SAFETY: see `mul`.
            unsafe {
                core::arch::asm!(
                    "subss {r}, {b}",
                    r = inout(xmm_reg) r,
                    b = in(xmm_reg) b,
                    options(pure, nomem, nostack, preserves_flags),
                );
            }
            r
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a - b
        }
    }

    /// `a / b` (`divss a, b`)
    #[inline]
    pub fn div(a: f32, b: f32) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            let mut r = a;
            // SAFETY: see `mul`.
            unsafe {
                core::arch::asm!(
                    "divss {r}, {b}",
                    r = inout(xmm_reg) r,
                    b = in(xmm_reg) b,
                    options(pure, nomem, nostack, preserves_flags),
                );
            }
            r
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            a / b
        }
    }
}

use ops::{add, div, mul, sub};

/// `static lm_vec2 lm_v2(float x, float y)`
#[inline]
fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

/// `static lm_vec2 lm_sub2(lm_vec2 a, lm_vec2 b)`
#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(sub(a.x, b.x), sub(a.y, b.y))
}

/// `lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p)`
///
/// The sequence of operations matches the C exactly, including the
/// reciprocal-then-multiply form (`invDenom = 1.0f / denom` followed by two
/// multiplies) rather than a direct division, so the single-precision rounding
/// is identical. A degenerate triangle gives a zero denominator and hence
/// infinite/NaN components exactly as the C does; that behaviour is reproduced,
/// not "fixed".
#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);

    // dot00 = lm_dot2(v0, v0)
    let dot00 = add(mul(v0.x, v0.x), mul(v0.y, v0.y));
    // dot01 = lm_dot2(v0, v1); both products use the v1 component as destination
    let dot01 = add(mul(v1.x, v0.x), mul(v1.y, v0.y));
    // dot02 = lm_dot2(v0, v2); the addition uses the y product as destination
    let dot02 = add(mul(v0.y, v2.y), mul(v0.x, v2.x));
    // dot11 = lm_dot2(v1, v1)
    let dot11 = add(mul(v1.x, v1.x), mul(v1.y, v1.y));
    // dot12 = lm_dot2(v1, v2); the y product uses the v2 component as destination
    let dot12 = add(mul(v1.x, v2.x), mul(v2.y, v1.y));

    // invDenom = 1.0f / (dot00 * dot11 - dot01 * dot01)
    let denom = sub(mul(dot11, dot00), mul(dot01, dot01));
    let inv_denom = div(1.0f32, denom);

    // u = (dot11 * dot02 - dot01 * dot12) * invDenom
    let u = mul(sub(mul(dot11, dot02), mul(dot12, dot01)), inv_denom);
    // v = (dot00 * dot12 - dot01 * dot02) * invDenom
    let v = mul(sub(mul(dot12, dot00), mul(dot02, dot01)), inv_denom);

    lm_v2(u, v)
}
