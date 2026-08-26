//! Rust translation of the C library in `c_src/`.
//!
//! The complete public ABI of the C shared library (per `nm -D`) is a single
//! exported symbol:
//!
//!   * `to_barycentric`
//!
//! The public header declares:
//!
//! ```c
//! typedef struct lm_vec2 { float x, y; } lm_vec2;
//! lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p);
//! ```
//!
//! `lm_v2`, `lm_sub2` and `lm_dot2` are `static` in the C translation unit, so
//! they are deliberately *not* exported here either.
//!
//! # Bit-exactness
//!
//! Every value is computed in single precision (`f32`) in exactly the same
//! order as the C source, so ordinary results are bit-identical.
//!
//! For NaN operands the *sign and payload* of the result additionally depend on
//! which operand the x86 SSE instruction uses as its destination: `mulss`,
//! `addss`, `subss` and `divss` return the destination operand (quieted) when
//! it is NaN, and only otherwise propagate the source operand. When both
//! operands are NaN the destination therefore wins, and the two NaNs may have
//! different signs -- x86 manufactures the "indefinite" QNaN `0xFFC00000`
//! (negative) for invalid operations such as `0 * inf` or `inf - inf`, while a
//! propagated NaN usually keeps a positive sign.
//!
//! The reference C library is built by `c_src/CMakeLists.txt` with no
//! `CMAKE_BUILD_TYPE`, i.e. unoptimized, and its codegen does *not* uniformly
//! use the left operand as the destination. From the reference disassembly of
//! `lm_dot2` (`a.x * b.x + a.y * b.y`):
//!
//! ```text
//! mulss  %xmm0,%xmm1     ; xmm1 = a.x * b.x   -> destination is a.x  (left)
//! mulss  %xmm2,%xmm0     ; xmm0 = b.y * a.y   -> destination is b.y  (RIGHT)
//! addss  %xmm1,%xmm0     ; xmm0 = (b.y*a.y) + (a.x*b.x) -> destination is the
//!                        ;                                 y term   (RIGHT)
//! ```
//!
//! A plain Rust `a.x * b.x + a.y * b.y` lets LLVM choose the commuted operand
//! order, which flips the sign of the resulting NaN for some inputs. To stay
//! byte-identical, the arithmetic below is expressed through primitives that
//! pin the destination operand via inline assembly, mirroring the reference
//! codegen instruction for instruction.

#![allow(non_camel_case_types)]

/// Mirrors `typedef struct lm_vec2 { float x, y; } lm_vec2;`
///
/// An 8-byte `repr(C)` struct of two `float`s, which the System V AMD64 ABI
/// classifies as SSE and therefore passes and returns in the low half of a
/// single XMM register -- matching the C library exactly.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

// ---------------------------------------------------------------------------
// Scalar primitives with an explicitly pinned destination operand.
//
// Each `*_ss(d, s)` computes `d <op> s` with `d` as the x86 destination
// operand, which fixes NaN selection exactly as the reference build does.
// ---------------------------------------------------------------------------

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
macro_rules! sse_op {
    ($name:ident, $mnemonic:literal) => {
        #[inline(always)]
        pub(super) fn $name(d: f32, s: f32) -> f32 {
            let mut d = d;
            // SAFETY: a single side-effect-free SSE arithmetic instruction on
            // two scalar float register operands. SSE is baseline on x86_64.
            unsafe {
                core::arch::asm!(
                    concat!($mnemonic, " {d}, {s}"),
                    d = inout(xmm_reg) d,
                    s = in(xmm_reg) s,
                    options(pure, nomem, nostack, preserves_flags),
                );
            }
            d
        }
    };
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
mod ops {
    sse_op!(mul_ss, "mulss");
    sse_op!(add_ss, "addss");
    sse_op!(sub_ss, "subss");
    sse_op!(div_ss, "divss");
}

/// Portable fallback for non-x86 targets: numerically identical, but NaN
/// sign/payload selection is left to the target's own FP semantics.
#[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
mod ops {
    #[inline(always)]
    pub(super) fn mul_ss(d: f32, s: f32) -> f32 {
        d * s
    }
    #[inline(always)]
    pub(super) fn add_ss(d: f32, s: f32) -> f32 {
        d + s
    }
    #[inline(always)]
    pub(super) fn sub_ss(d: f32, s: f32) -> f32 {
        d - s
    }
    #[inline(always)]
    pub(super) fn div_ss(d: f32, s: f32) -> f32 {
        d / s
    }
}

use ops::{add_ss, div_ss, mul_ss, sub_ss};

// ---------------------------------------------------------------------------
// Translations of the C translation unit's `static` helpers.
// ---------------------------------------------------------------------------

/// `static lm_vec2 lm_v2(float x, float y)`
#[inline(always)]
fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

/// `static lm_vec2 lm_sub2(lm_vec2 a, lm_vec2 b)` -- both subtractions use the
/// left operand as the destination, as in the reference codegen.
#[inline(always)]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(sub_ss(a.x, b.x), sub_ss(a.y, b.y))
}

/// `static float lm_dot2(lm_vec2 a, lm_vec2 b)` = `a.x * b.x + a.y * b.y`.
///
/// The reference codegen computes the `y` product with `b.y` as the destination
/// and then adds with that `y` term as the destination, so the operand order is
/// reproduced here rather than written in the naive left-to-right form.
#[inline(always)]
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    let xx = mul_ss(a.x, b.x);
    let yy = mul_ss(b.y, a.y);
    add_ss(yy, xx)
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// `lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p)`
///
/// Note: the C code divides without guarding against a zero determinant, so
/// degenerate triangles yield infinities / NaNs. That behaviour is reproduced
/// verbatim -- it is not "fixed" here.
#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);

    // invDenom = 1.0f / (dot00 * dot11 - dot01 * dot01)
    let inv_denom = div_ss(
        1.0f32,
        sub_ss(mul_ss(dot00, dot11), mul_ss(dot01, dot01)),
    );
    // u = (dot11 * dot02 - dot01 * dot12) * invDenom
    let u = mul_ss(
        sub_ss(mul_ss(dot11, dot02), mul_ss(dot01, dot12)),
        inv_denom,
    );
    // v = (dot00 * dot12 - dot01 * dot02) * invDenom
    let v = mul_ss(
        sub_ss(mul_ss(dot00, dot12), mul_ss(dot01, dot02)),
        inv_denom,
    );
    lm_v2(u, v)
}
