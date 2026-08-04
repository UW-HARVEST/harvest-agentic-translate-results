//! Rust translation of the C `match` library.
//!
//! Translated from `c_src/`:
//!   * `include/match.h`
//!   * `src/match.c`
//!   * `src/spectral_contrast.c`

use std::os::raw::c_int;

/// Size of smoothing kernel.
pub const N_SMOOTH: usize = 16;

/// Desired precision for floating-point vectors.
#[allow(non_camel_case_types)]
pub type float_t = f64;

// -- spectral_contrast.c --------------------------------------------------

fn dot_product(a: &[float_t], b: &[float_t], length: usize) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..length {
        sum += a[i] * b[i];
    }
    sum
}

fn normalize(v: &mut [float_t], length: usize) {
    let magnitude = dot_product(v, v, length).sqrt();
    for i in 0..length {
        v[i] /= magnitude;
    }
}

/// Computes the spectral contrast (normalized dot product) between two
/// vectors of equal `length`. Note: `a` and `b` are normalized in place,
/// matching the original C semantics where the input arrays are mutated.
pub fn spectral_contrast(a: &mut [float_t], b: &mut [float_t], length: usize) -> f64 {
    normalize(a, length);
    normalize(b, length);
    dot_product(a, b, length)
}

// -- match.c --------------------------------------------------------------

fn total(v: &[float_t], length: usize) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..length {
        sum += v[i];
    }
    sum
}

fn smoothen(v: &mut [float_t], length: usize) {
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j = 0usize;
        while j < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / (N_SMOOTH as f64);
    }
}

fn differentiate(v: &mut [float_t], length: usize) {
    if length == 0 {
        return;
    }
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(v: &mut [float_t], source: &[float_t], length: usize) {
    v[..length].copy_from_slice(&source[..length]);
    smoothen(v, length);
    differentiate(v, length);
    smoothen(v, length);
}

/// Returns 1 if `test` matches `reference` (within `threshold`), 0 otherwise.
pub fn match_vectors(
    test: &[float_t],
    reference: &[float_t],
    bins: usize,
    threshold: f64,
) -> i32 {
    if total(test, bins) < threshold * total(reference, bins) {
        return 0;
    }
    let mut t: Vec<float_t> = vec![0.0; bins];
    let mut r: Vec<float_t> = vec![0.0; bins];
    preprocess(&mut t, test, bins);
    preprocess(&mut r, reference, bins);
    if spectral_contrast(&mut t, &mut r, bins) >= threshold {
        1
    } else {
        0
    }
}

// -- C-compatible FFI exports --------------------------------------------

/// C ABI: `int match(float_t *test, float_t *reference, int bins, double threshold);`
///
/// # Safety
///
/// `test` and `reference` must each point to at least `bins` valid `f64`
/// values. `bins` must be non-negative.
#[no_mangle]
pub unsafe extern "C" fn r#match(
    test: *mut float_t,
    reference: *mut float_t,
    bins: c_int,
    threshold: f64,
) -> c_int {
    if bins < 0 || test.is_null() || reference.is_null() {
        return 0;
    }
    let n = bins as usize;
    let test_slice = std::slice::from_raw_parts(test, n);
    let ref_slice = std::slice::from_raw_parts(reference, n);
    match_vectors(test_slice, ref_slice, n, threshold) as c_int
}

/// C ABI: `double spectral_contrast(float_t *a, float_t *b, int length);`
///
/// # Safety
///
/// `a` and `b` must each point to at least `length` valid `f64` values
/// that are safe to mutate. `length` must be non-negative.
#[export_name = "spectral_contrast"]
pub unsafe extern "C" fn spectral_contrast_ffi(
    a: *mut float_t,
    b: *mut float_t,
    length: c_int,
) -> f64 {
    if length < 0 || a.is_null() || b.is_null() {
        return 0.0;
    }
    let n = length as usize;
    let a_slice = std::slice::from_raw_parts_mut(a, n);
    let b_slice = std::slice::from_raw_parts_mut(b, n);
    spectral_contrast(a_slice, b_slice, n)
}
