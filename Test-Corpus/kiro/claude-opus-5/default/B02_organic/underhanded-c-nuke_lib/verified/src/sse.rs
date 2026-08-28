//! Scalar SSE arithmetic with x86 NaN-propagation semantics.
//!
//! IEEE 754 leaves the payload of a NaN result unspecified, so a C compiler and
//! `rustc` are both free to pick either operand's payload when more than one
//! input is a NaN. The hardware, however, is not: for a scalar SSE2 binary
//! operation `OP DEST, SRC`, Intel's rules are
//!
//! * if `DEST` is a NaN, the result is `DEST` with the quiet bit forced on;
//! * otherwise, if `SRC` is a NaN, the result is `SRC` with the quiet bit forced
//!   on;
//! * otherwise the operation proceeds normally (and may still raise *invalid*
//!   and yield the default QNaN, e.g. `inf - inf`).
//!
//! GCC at the optimisation level used for `c_src` emits, for `sum += v[i]`,
//! `addsd %xmm_sum, %xmm_v` -- that is, the *loaded element* is `DEST` and the
//! running accumulator is `SRC`. Writing `sum + v[i]` in Rust gives the opposite
//! assignment, so a NaN accumulator would win over a NaN element and the
//! returned `double` would differ in its payload bits. `spectral_contrast`
//! returns that `double` straight to the caller, so the difference is
//! observable.
//!
//! These helpers pin the choice down explicitly instead of relying on operand
//! order surviving codegen. The non-NaN paths are ordinary IEEE operations, so
//! results for finite inputs are unaffected.

/// `f64` quiet bit (mantissa MSB).
const QUIET_F64: u64 = 1 << 51;
/// `f32` quiet bit (mantissa MSB).
const QUIET_F32: u32 = 1 << 22;

/// Applies the NaN-selection rules above, returning `None` when neither operand
/// is a NaN and the arithmetic should simply be performed.
#[inline(always)]
fn nan_result_f64(dest: f64, src: f64) -> Option<f64> {
    if dest.is_nan() {
        Some(f64::from_bits(dest.to_bits() | QUIET_F64))
    } else if src.is_nan() {
        Some(f64::from_bits(src.to_bits() | QUIET_F64))
    } else {
        None
    }
}

/// `f32` counterpart of [`nan_result_f64`].
#[inline(always)]
fn nan_result_f32(dest: f32, src: f32) -> Option<f32> {
    if dest.is_nan() {
        Some(f32::from_bits(dest.to_bits() | QUIET_F32))
    } else if src.is_nan() {
        Some(f32::from_bits(src.to_bits() | QUIET_F32))
    } else {
        None
    }
}

/// `ADDSD dest, src`.
#[inline(always)]
pub fn addsd(dest: f64, src: f64) -> f64 {
    match nan_result_f64(dest, src) {
        Some(nan) => nan,
        None => dest + src,
    }
}

/// `MULSD dest, src`.
#[inline(always)]
pub fn mulsd(dest: f64, src: f64) -> f64 {
    match nan_result_f64(dest, src) {
        Some(nan) => nan,
        None => dest * src,
    }
}

/// `DIVSD dest, src`, computing `dest / src`.
#[inline(always)]
pub fn divsd(dest: f64, src: f64) -> f64 {
    match nan_result_f64(dest, src) {
        Some(nan) => nan,
        None => dest / src,
    }
}

/// `SUBSD dest, src`, computing `dest - src`.
#[inline(always)]
pub fn subsd(dest: f64, src: f64) -> f64 {
    match nan_result_f64(dest, src) {
        Some(nan) => nan,
        None => dest - src,
    }
}

/// `MULSS dest, src`.
#[inline(always)]
pub fn mulss(dest: f32, src: f32) -> f32 {
    match nan_result_f32(dest, src) {
        Some(nan) => nan,
        None => dest * src,
    }
}
