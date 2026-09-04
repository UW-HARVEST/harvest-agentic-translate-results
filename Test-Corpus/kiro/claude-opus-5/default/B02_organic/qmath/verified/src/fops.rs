//! Scalar single-precision arithmetic with the exact operand-ordering
//! semantics that the C program gets from x86-64 SSE (`addss`, `subss`,
//! `mulss`).
//!
//! Rust/LLVM is free to commute the operands of a floating point addition or
//! multiplication, and it does: `a*b + c*d + e*f` comes out with the adds'
//! sources swapped relative to what gcc emits for the same expression. For
//! ordinary values this is invisible, but NaN propagation is *not*
//! commutative on x86, so the swap changes which NaN (and therefore which
//! sign) survives. The C output is the ground truth, so the operand order has
//! to be pinned down explicitly.
//!
//! The rule, verified against the compiled C program on this target:
//!
//! * if the left operand is a NaN, the result is that NaN, quieted, with its
//!   sign and payload preserved;
//! * otherwise, if the right operand is a NaN, the result is that NaN,
//!   quieted;
//! * otherwise the hardware operation is performed, and if that operation is
//!   invalid (`inf - inf`, `0 * inf`) the result is the x86 "QNaN floating
//!   point indefinite", which is negative: `0xFFC0_0000`.

/// The x86 single-precision QNaN floating-point indefinite, produced by an
/// invalid operation whose operands are not themselves NaNs. Note the set sign
/// bit: `printf("%f")` renders it as `-nan`.
const QNAN_INDEFINITE: u32 = 0xFFC0_0000;

/// Turn a signaling NaN into the corresponding quiet NaN, leaving an already
/// quiet NaN untouched. This is what the hardware does when it propagates a
/// NaN operand.
#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// Shared NaN bookkeeping for the three operations below.
#[inline]
fn propagate(a: f32, b: f32, result: f32) -> f32 {
    if a.is_nan() {
        return quiet(a);
    }
    if b.is_nan() {
        return quiet(b);
    }
    if result.is_nan() {
        // Neither operand was a NaN, so this is an invalid operation.
        return f32::from_bits(QNAN_INDEFINITE);
    }
    result
}

/// `addss a, b` — `a` is the destination operand.
#[inline]
pub fn fadd(a: f32, b: f32) -> f32 {
    propagate(a, b, a + b)
}

/// `subss a, b` — `a` is the destination operand.
#[inline]
pub fn fsub(a: f32, b: f32) -> f32 {
    propagate(a, b, a - b)
}

/// `mulss a, b` — `a` is the destination operand.
#[inline]
pub fn fmul(a: f32, b: f32) -> f32 {
    propagate(a, b, a * b)
}
