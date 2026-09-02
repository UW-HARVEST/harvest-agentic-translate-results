//! Rust translation of `c_src/src/match.c`.
//!
//! This translation unit sees `match.h`, so here `float_t` is `double` (`f64`).
//! `spectral_contrast`, however, was compiled against `<math.h>`'s `float_t`
//! (`f32`) -- see `spectral_contrast.rs`. The C code therefore hands
//! `double`-typed scratch buffers to a function that reads them as `float`.
//! That reinterpretation is part of the observable behaviour and is reproduced
//! verbatim below.

use std::ffi::c_int;
use std::slice;

use crate::fp::{add_sd, mul_sd};
use crate::spectral_contrast::spectral_contrast_slices;

/// `#define N_SMOOTH 16` from `include/match.h`.
const N_SMOOTH: usize = 16;

/// `static double total(float_t *v, int length)`
fn total(v: &[f64]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..v.len() {
        sum = add_sd(sum, v[i]);
    }
    sum
}

/// `static void smoothen(float_t *v, int length)`
///
/// In-place box filter with a truncated (not wrapped, not renormalised) kernel
/// at the tail: the divisor is always `N_SMOOTH`, even when fewer than
/// `N_SMOOTH` samples were available. Reproduced as-is.
fn smoothen(v: &mut [f64]) {
    let length = v.len();
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j = 0usize;
        while j < N_SMOOTH && i + j < length {
            sum = add_sd(sum, v[i + j]);
            j += 1;
        }
        v[i] = sum / N_SMOOTH as f64;
    }
}

/// `static void differentiate(float_t *v, int length)`
fn differentiate(v: &mut [f64]) {
    let length = v.len();
    // C: `for(i = 0; i < length - 1; i++)` then `v[length - 1] = 0;`
    // For length == 0 the C writes `v[-1]`, an out-of-bounds store into stack
    // padding of the caller's VLA. It is unreachable from any observable
    // output (the buffer is function-local and `spectral_contrast` returns 0.0
    // for length 0), so it is elided rather than reproduced as UB.
    if length == 0 {
        return;
    }
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

/// `static void preprocess(float_t *v, float_t *source, int length)`
fn preprocess(v: &mut [f64], source: &[f64]) {
    v.copy_from_slice(source); // memcpy(v, source, length * sizeof(*v))
    smoothen(v);
    differentiate(v);
    smoothen(v);
}

/// `int match(float_t *test, float_t *reference, int bins, double threshold)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut f64,
    reference: *mut f64,
    bins: c_int,
    threshold: f64,
) -> c_int {
    // `float_t t[bins], r[bins];` -- a VLA. Every loop in this TU is bounded by
    // `i < length`, so a non-positive `bins` behaves as an empty buffer.
    let n = if bins > 0 { bins as usize } else { 0 };

    let test_in: &[f64] = if n == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(test, n) }
    };
    let reference_in: &[f64] = if n == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(reference, n) }
    };

    // Error/validation order is preserved: the energy gate runs first and
    // short-circuits before any preprocessing.
    // GCC evaluates the right-hand side as `mulsd total_ref, threshold`, so
    // `total(reference)` is the multiply's destination operand.
    if total(test_in) < mul_sd(total(reference_in), threshold) {
        return 0;
    }

    let mut t = vec![0.0f64; n];
    let mut r = vec![0.0f64; n];
    preprocess(&mut t, test_in);
    preprocess(&mut r, reference_in);

    // `spectral_contrast(t, r, bins)`: match.h declares the parameters as
    // `float_t *` == `double *`, but the definition was compiled with
    // `float_t` == `float`. The callee consequently reads `bins` *f32* lanes
    // out of the low half of each f64 buffer, and writes its normalised f32
    // results back over those same bytes. Reproduce the reinterpretation.
    //
    // `Vec<f64>` is 8-byte aligned, so the f32 view is well aligned, and
    // `bins` f32 lanes fit inside `bins` f64 slots.
    let contrast = {
        let tf: &mut [f32] = if n == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(t.as_mut_ptr() as *mut f32, n) }
        };
        let rf: &mut [f32] = if n == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(r.as_mut_ptr() as *mut f32, n) }
        };
        spectral_contrast_slices(tf, rf)
    };

    (contrast >= threshold) as c_int
}
