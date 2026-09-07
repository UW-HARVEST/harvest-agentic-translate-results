//! Rust translation of `c_src/src/match.c`.
//!
//! This translation unit sees `match.h`, so here `float_t` is `double` (`f64`).
//! `spectral_contrast`, however, was compiled against `<math.h>`'s `float_t`
//! (`f32`) -- see `spectral_contrast.rs`. The C code therefore hands
//! `double`-typed scratch buffers to a function that reads them as `float`,
//! with a 4-byte stride: the `bins` lanes it sees are the interleaved low and
//! high halves of the first `ceil(bins / 2)` doubles. That reinterpretation is
//! part of the observable behaviour and is reproduced verbatim below.
//!
//! The caller-supplied `test` / `reference` arrays are walked through raw
//! pointers rather than slices, so that a non-positive `bins` never
//! dereferences them and a `NULL` pointer with a positive `bins` faults exactly
//! where the C faults (see `spectral_contrast.rs` for the full rationale).

use std::ffi::c_int;
use std::ptr;

use crate::fp::{add_sd, mul_sd};
use crate::spectral_contrast::spectral_contrast_raw;

/// `#define N_SMOOTH 16` from `include/match.h`.
const N_SMOOTH: usize = 16;

/// `static double total(float_t *v, int length)`
///
/// GCC (-O0) emits `movsd v[i],%xmm0` / `movsd sum,%xmm1` /
/// `addsd %xmm1,%xmm0`, i.e. the ADDSD *destination* is `v[i]` and the *source*
/// is the running `sum`. See `crate::fp` for why that role assignment matters.
unsafe fn total(v: *const f64, length: c_int) -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: c_int = 0;
    while i < length {
        let x = unsafe { *v.offset(i as isize) };
        sum = add_sd(x, sum);
        i += 1;
    }
    sum
}

/// `static void smoothen(float_t *v, int length)`
///
/// In-place box filter with a truncated (not wrapped, not renormalised) kernel
/// at the tail: the divisor is always `N_SMOOTH`, even when fewer than
/// `N_SMOOTH` samples were available. Reproduced as-is.
///
/// As in `total`, GCC's ADDSD destination is the freshly loaded `v[i + j]` and
/// the source is the running `sum`.
fn smoothen(v: &mut [f64]) {
    let length = v.len();
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j = 0usize;
        while j < N_SMOOTH && i + j < length {
            sum = add_sd(v[i + j], sum);
            j += 1;
        }
        v[i] = sum / N_SMOOTH as f64;
    }
}

/// `static void differentiate(float_t *v, int length)`
fn differentiate(v: &mut [f64]) {
    let length = v.len();
    // C: `for(i = 0; i < length - 1; i++)` then `v[length - 1] = 0;`
    //
    // For `length == 0` the C writes `v[-1]`. Reached only from `match` with
    // `bins == 0`, where GCC's zero-length VLA leaves `rsp` unchanged, so that
    // store lands on `preprocess`'s saved return address and the C `.so`
    // reproducibly SIGSEGVs (see `ERRORS.md` rows 4 and 9). There is no value to
    // reproduce, so the store is elided rather than turned into Rust UB.
    if length == 0 {
        return;
    }
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

/// `static void preprocess(float_t *v, float_t *source, int length)`
unsafe fn preprocess(v: &mut [f64], source: *const f64) {
    // memcpy(v, source, length * sizeof(*v)); `v` is a fresh buffer, so the
    // no-overlap precondition of `memcpy` holds. Skipped for an empty buffer:
    // the C would call `memcpy(v, NULL, 0)` there, and `copy_nonoverlapping`
    // rejects a null source even for a zero count.
    if !v.is_empty() {
        unsafe { ptr::copy_nonoverlapping(source, v.as_mut_ptr(), v.len()) };
    }
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
    // Error/validation order is preserved: the energy gate runs first and
    // short-circuits before any preprocessing.
    // GCC evaluates the right-hand side as `mulsd total_ref, threshold`, so
    // `total(reference)` is the multiply's destination operand.
    let total_test = unsafe { total(test, bins) };
    let total_ref = unsafe { total(reference, bins) };
    if total_test < mul_sd(total_ref, threshold) {
        return 0;
    }

    // `float_t t[bins], r[bins];` -- a VLA. Every loop in this TU is bounded by
    // `i < length`, so a non-positive `bins` behaves as an empty buffer. (In the
    // C a non-positive `bins` is undefined behaviour that crashes; see
    // `ERRORS.md` rows 4 and 5.)
    let n = if bins > 0 { bins as usize } else { 0 };
    let mut t = vec![0.0f64; n];
    let mut r = vec![0.0f64; n];
    unsafe {
        preprocess(&mut t, test);
        preprocess(&mut r, reference);
    }

    // `spectral_contrast(t, r, bins)`: match.h declares the parameters as
    // `float_t *` == `double *`, but the definition was compiled with
    // `float_t` == `float`. The callee therefore walks each buffer with a
    // 4-byte stride: lane `2k` is the low half of `t[k]` and lane `2k + 1` is
    // its high half, so `bins` lanes span only the first `ceil(bins / 2)`
    // doubles, and the normalised `float` results are written back over exactly
    // those bytes. Reproduce the reinterpretation verbatim.
    //
    // `Vec<f64>` is 8-byte aligned, so the f32 view is well aligned, and
    // `bins` f32 lanes (4 * bins bytes) fit inside `bins` f64 slots.
    let contrast = unsafe {
        spectral_contrast_raw(t.as_mut_ptr() as *mut f32, r.as_mut_ptr() as *mut f32, bins)
    };

    (contrast >= threshold) as c_int
}
