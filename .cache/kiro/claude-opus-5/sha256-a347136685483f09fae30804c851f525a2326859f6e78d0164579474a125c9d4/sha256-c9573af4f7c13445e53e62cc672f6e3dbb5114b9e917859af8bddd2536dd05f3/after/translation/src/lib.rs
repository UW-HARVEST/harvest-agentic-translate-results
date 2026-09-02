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
//!    numerically irrelevant otherwise.
//!
//! Ordinary Rust source order does not survive LLVM's canonicalisation of
//! commutative operands, so on x86-64 each arithmetic step is emitted as a
//! single explicit instruction via `asm!`, with the destination register chosen
//! to match the instruction GCC actually emitted (annotated at each call site).
//! Only commutative operands are reordered relative to the C source, so every
//! non-NaN result is unchanged; subtraction and division appear in source order.
//!
//! On non-x86-64 targets the same operations fall back to plain Rust float
//! arithmetic, which is identical except for NaN payload provenance.
//!
//! ## Which C build is the reference
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the documented build
//!
//! ```text
//! cd c_src && mkdir -p build && cd build \
//!   && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
//! ```
//!
//! compiles at `-O0`, with no inlining. That artifact is the reference this
//! translation is byte-matched against, and it is the artifact the differential
//! tests in `tests/` load.
//!
//! This matters, because GCC picks *different* destination operands for the
//! commutative multiplies and additions at `-O0` and at `-O2`/`-O3`: the two C
//! builds are not bit-identical to each other. Measured with the fuzz row of
//! the test suite, they disagree on roughly 135 of 200 000 uniformly random
//! 32-bit argument patterns — exactly the cases where two *different* NaNs meet
//! as the two operands of one commutative instruction. No single implementation
//! can match both, so the documented (`-O0`) build was chosen. The tests accept
//! a `C_SO=<path>` override, which is how the optimized build was compared.
//!
//! Concretely, at `-O0` GCC emits `lm_dot2` as a real call whose body is
//! `add(mul(b.y, a.y), mul(a.x, b.x))`, and every operation in the body of
//! `to_barycentric` in plain source order. At `-O2`/`-O3` it inlines `lm_dot2`
//! and additionally swaps the operands of `dot00 * dot11`, `dot01 * dot12` and
//! both products of the `v` numerator.

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
///
/// GCC emits the `y` lane first and the `x` lane second, both with the `a`
/// component as the destination operand (`subss` is non-commutative, so the
/// operand order is fixed by the source in any case).
#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(sub(a.x, b.x), sub(a.y, b.y))
}

/// `static float lm_dot2(lm_vec2 a, lm_vec2 b)`
///
/// ```c
/// return a.x * b.x + a.y * b.y;
/// ```
///
/// The observed `-O0` codegen is
///
/// ```text
/// movss -0x8(%rbp),%xmm1   # a.x
/// movss -0x10(%rbp),%xmm0  # b.x
/// mulss %xmm0,%xmm1        # xmm1 = a.x * b.x   (dest = a.x)
/// movss -0x4(%rbp),%xmm2   # a.y
/// movss -0xc(%rbp),%xmm0   # b.y
/// mulss %xmm2,%xmm0        # xmm0 = b.y * a.y   (dest = b.y  <- swapped)
/// addss %xmm1,%xmm0        # xmm0 = ypro + xpro (dest = ypro <- swapped)
/// ```
///
/// so GCC picks `b.y` as the destination of the second multiply and the `y`
/// product as the destination of the addition. Both swaps are numerically
/// irrelevant but observable through NaN provenance, hence they are mirrored.
#[inline]
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    let prod_x = mul(a.x, b.x);
    let prod_y = mul(b.y, a.y);
    add(prod_y, prod_x)
}

/// `lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p)`
///
/// The sequence of operations matches the C exactly, including the
/// reciprocal-then-multiply form (`invDenom = 1.0f / denom` followed by two
/// multiplies) rather than a direct division, so the single-precision rounding
/// is identical. A degenerate triangle gives a zero denominator and hence
/// infinite/NaN components exactly as the C does; that behaviour is reproduced,
/// not "fixed".
///
/// Every arithmetic step in this function body is emitted by GCC in plain
/// source order with the left operand as the SSE destination register, so no
/// operand swapping is needed here — only inside [`lm_dot2`].
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
    let denom = sub(mul(dot00, dot11), mul(dot01, dot01));
    let inv_denom = div(1.0f32, denom);

    // u = (dot11 * dot02 - dot01 * dot12) * invDenom
    let u = mul(sub(mul(dot11, dot02), mul(dot01, dot12)), inv_denom);
    // v = (dot00 * dot12 - dot01 * dot02) * invDenom
    let v = mul(sub(mul(dot00, dot12), mul(dot01, dot02)), inv_denom);

    lm_v2(u, v)
}
