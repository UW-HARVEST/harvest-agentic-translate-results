//! x86-64 SSE-faithful floating-point primitives.
//!
//! For finite/infinite operands these are plain IEEE-754 operations and the
//! helpers below are indistinguishable from `+` / `*`. They exist purely to pin
//! down **NaN payload propagation**, which IEEE-754 leaves unspecified and
//! which therefore differs between the GCC-compiled C and LLVM-compiled Rust.
//!
//! SSE rules reproduced here:
//!   * For `ADDSD dst, src` / `MULSS dst, src`, if the *destination* operand is
//!     a NaN the result is that NaN, quieted; only if the destination is not a
//!     NaN does the *source* NaN propagate.
//!   * "Quieting" sets the most significant significand bit and leaves the sign
//!     and the remaining payload bits alone.
//!
//! Why this matters: `fadd` and `fmul` are commutative, so LLVM is free to pick
//! either operand as the two-address destination. It consistently chooses the
//! dying operand (the freshly loaded value), whereas GCC consistently keeps the
//! accumulator in the destination register. With NaN inputs the two choices
//! yield different payloads in the returned `double`. Reproducing the C exactly
//! requires pinning the operand roles.
//!
//! Non-commutative operations (`SUBSD`, `DIVSD`, `DIVPD`, `SQRTSD`) need no
//! helper: their destination operand is fixed by the source order, and LLVM
//! cannot swap it.

#[inline(always)]
fn quiet_f64(x: f64) -> f64 {
    f64::from_bits(x.to_bits() | 0x0008_0000_0000_0000)
}

#[inline(always)]
fn quiet_f32(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `ADDSD dst, src` -> `dst + src`, with the destination NaN winning.
#[inline(always)]
pub(crate) fn add_sd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet_f64(dst)
    } else if src.is_nan() {
        quiet_f64(src)
    } else {
        dst + src
    }
}

/// `MULSD dst, src` -> `dst * src`, with the destination NaN winning.
#[inline(always)]
pub(crate) fn mul_sd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet_f64(dst)
    } else if src.is_nan() {
        quiet_f64(src)
    } else {
        dst * src
    }
}

/// `MULSS dst, src` -> `dst * src` in single precision, destination NaN winning.
#[inline(always)]
pub(crate) fn mul_ss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_f32(dst)
    } else if src.is_nan() {
        quiet_f32(src)
    } else {
        dst * src
    }
}
