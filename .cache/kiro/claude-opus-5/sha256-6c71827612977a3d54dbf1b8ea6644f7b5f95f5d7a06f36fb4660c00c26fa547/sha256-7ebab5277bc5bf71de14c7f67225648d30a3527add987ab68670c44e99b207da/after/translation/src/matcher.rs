//! Translation of `c_src/src/match.c`.
//!
//! This translation unit includes `match.h`, so `float_t` here is `double`.

use core::ffi::{c_double, c_int};

use crate::spectral_contrast::spectral_contrast;
use crate::sse::{addsd, divsd, mulsd, subsd};
use crate::{clamp_len, slice_from_raw};

/// `#define N_SMOOTH 16` -- size of the smoothing kernel.
const N_SMOOTH: usize = 16;

/// Scratch buffers here carry one extra leading element ahead of the logical
/// vector `v`, so the index this offset applies is spelled out once.
const PAD: usize = 1;

/// ```c
/// static double total(float_t *v, int length) {
///     double sum = 0;
///     int i;
///     for(i = 0; i < length; i++) sum += v[i];
///     return sum;
/// }
/// ```
///
/// `addsd` takes the loaded element as its destination and the accumulator as
/// its source, which decides the payload when both are NaN.
fn total(v: &[f64]) -> f64 {
    let mut sum: f64 = 0.0;
    for &element in v {
        sum = addsd(element, sum);
    }
    sum
}

/// ```c
/// static void smoothen(float_t *v, int length) {
///     double sum;
///     int i, j;
///     for(i = 0; i < length; i++) {
///         sum = 0;
///         for(j = 0; j < N_SMOOTH && i + j < length; j++)
///             sum += v[i + j];
///         v[i] = sum / N_SMOOTH;
///     }
/// }
/// ```
///
/// The tail elements (`i > length - N_SMOOTH`) still divide by the full
/// `N_SMOOTH` even though fewer terms were summed, which damps the tail towards
/// zero. That is the C's behaviour and is kept.
fn smoothen(v: &mut [f64]) {
    let length = v.len();
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j = 0usize;
        while j < N_SMOOTH && i + j < length {
            sum = addsd(v[i + j], sum);
            j += 1;
        }
        v[i] = divsd(sum, N_SMOOTH as f64);
    }
}

/// ```c
/// static void differentiate(float_t *v, int length) {
///     int i;
///     for(i = 0; i < length - 1; i++) v[i] = v[i + 1] - v[i];
///     v[length - 1] = 0;
/// }
/// ```
///
/// `buf` is a padded buffer: `buf[0]` is scratch and `buf[PAD..]` is the logical
/// vector `v` of length `buf.len() - PAD`. The padding exists so that the C's
/// `v[length - 1] = 0` store still has somewhere to land when `length == 0`,
/// where C writes one element *before* the array. Reproducing that store
/// harmlessly keeps the memory-safety of this translation without changing any
/// observable result.
fn differentiate(buf: &mut [f64]) {
    let length = buf.len() - PAD;
    for i in 0..length.saturating_sub(1) {
        buf[PAD + i] = subsd(buf[PAD + i + 1], buf[PAD + i]);
    }
    // `PAD + length - 1` for the in-bounds case, `buf[0]` (the pad) for
    // `length == 0`.
    buf[length] = 0.0;
}

/// ```c
/// static void preprocess(float_t *v, float_t *source, int length) {
///     memcpy(v, source, length * sizeof(*v));
///     smoothen(v, length);
///     differentiate(v, length);
///     smoothen(v, length);
/// }
/// ```
fn preprocess(buf: &mut [f64], source: &[f64]) {
    buf[PAD..].copy_from_slice(source);
    smoothen(&mut buf[PAD..]);
    differentiate(buf);
    smoothen(&mut buf[PAD..]);
}

/// ```c
/// int match(float_t *test, float_t *reference, int bins, double threshold) {
///     float_t t[bins], r[bins];
///     if(total(test, bins) < threshold * total(reference, bins)) return 0;
///     preprocess(t, test, bins);
///     preprocess(r, reference, bins);
///     return spectral_contrast(t, r, bins) >= threshold;
/// }
/// ```
///
/// `t` and `r` are `double` arrays, but `spectral_contrast` -- compiled with
/// `float_t == float` -- reinterprets them as `float` arrays. The cast below
/// preserves that. See the crate docs.
///
/// Both comparisons come out `false` when either side is a NaN, matching
/// `comisd`+`jbe` and `comisd`+`setae` in the C.
///
/// # Safety
/// `test` and `reference` must each be valid for reads of `bins` `double`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut c_double,
    reference: *mut c_double,
    bins: c_int,
    threshold: c_double,
) -> c_int {
    let len = clamp_len(bins);
    let test = unsafe { slice_from_raw(test, len) };
    let reference = unsafe { slice_from_raw(reference, len) };

    // `mulsd` has `total(reference, bins)` as its destination operand.
    if total(test) < mulsd(total(reference), threshold) {
        return 0;
    }

    let mut t = vec![0.0f64; len + PAD];
    let mut r = vec![0.0f64; len + PAD];
    preprocess(&mut t, test);
    preprocess(&mut r, reference);

    let contrast = unsafe {
        spectral_contrast(
            t[PAD..].as_mut_ptr().cast(),
            r[PAD..].as_mut_ptr().cast(),
            bins,
        )
    };
    (contrast >= threshold) as c_int
}
