//! Single-precision arithmetic with the exact operand-selection behaviour of
//! the SSE instructions gcc emits for this program.
//!
//! Plain Rust `*`, `+` and `-` on `f32` produce the right *numeric* result, but
//! two things about them are not observable-behaviour-preserving here:
//!
//! * When an operand is NaN, `mulss`/`addss`/`subss` return the **destination**
//!   operand's NaN if it is one, otherwise the source operand's. LLVM freely
//!   canonicalises the operand order of commutative float ops and may vectorise
//!   `v[0]*=k; v[1]*=k; v[2]*=k` into a single `mulps` with the scalar as the
//!   destination — either of which flips which NaN (and therefore which sign)
//!   comes out.
//! * LLVM's constant folder does not model the x86 "QNaN floating-point
//!   indefinite" result of an invalid operation, which has its **sign bit set**
//!   (`0xFFC00000`), so `inf * 0` prints as `-nan` from the C program.
//!
//! Making the operand order and the invalid-operation results explicit pins the
//! behaviour down regardless of how the optimiser rearranges things.

/// The x86 "QNaN floating-point indefinite" produced by an invalid operation.
const INDEFINITE: u32 = 0xFFC0_0000;

/// SSE quiets a signalling NaN operand before passing it through; the payload
/// and sign are otherwise preserved.
fn quiet(f: f32) -> f32 {
    f32::from_bits(f.to_bits() | 0x0040_0000)
}

/// `a * b`, where `a` is the destination operand (`mulss a, b`).
pub fn fmul(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        return quiet(a);
    }
    if b.is_nan() {
        return quiet(b);
    }
    // Invalid operation: infinity times zero.
    if (a.is_infinite() && b == 0.0) || (b.is_infinite() && a == 0.0) {
        return f32::from_bits(INDEFINITE);
    }
    a * b
}

/// `a + b`, where `a` is the destination operand (`addss a, b`).
pub fn fadd(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        return quiet(a);
    }
    if b.is_nan() {
        return quiet(b);
    }
    // Invalid operation: infinities of opposite sign.
    if a.is_infinite() && b.is_infinite() && a.is_sign_negative() != b.is_sign_negative() {
        return f32::from_bits(INDEFINITE);
    }
    a + b
}

/// `a - b`, where `a` is the destination operand (`subss a, b`).
pub fn fsub(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        return quiet(a);
    }
    if b.is_nan() {
        return quiet(b);
    }
    // Invalid operation: infinities of like sign.
    if a.is_infinite() && b.is_infinite() && a.is_sign_negative() == b.is_sign_negative() {
        return f32::from_bits(INDEFINITE);
    }
    a - b
}
