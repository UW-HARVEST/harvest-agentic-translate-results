// Translation of c_src/src/match.c and c_src/src/spectral_contrast.c
//
// IMPORTANT BUG-PRESERVATION NOTE:
//
// The C source has a subtle, ABI-relevant bug that this translation reproduces
// byte-for-byte:
//
//   * `match.h` declares `typedef double float_t;` and prototypes
//     `double spectral_contrast(float_t *a, float_t *b, int length);`
//     so callers using the header pass `double *`.
//
//   * `spectral_contrast.c` does NOT include `match.h`. It only includes
//     `<math.h>`, where `float_t` is the typedef for `float` (when
//     FLT_EVAL_METHOD == 0, which is the case on common x86_64 glibc).
//
//   * Therefore the actual implementation of `spectral_contrast` operates on
//     `float *` (4-byte elements), even though `match.c` (which DOES include
//     `match.h`) calls it with `double *` (8-byte elements) arrays.
//
// This means `spectral_contrast` reinterprets the first `length` 4-byte words
// of the double-precision array as IEEE-754 single-precision floats, and
// `normalize` mutates them in place as floats. We faithfully reproduce that
// behavior here.

use std::ffi::c_int;

const N_SMOOTH: c_int = 16;

// ---------- match.c ----------

fn total(v: &[f64]) -> f64 {
    let mut sum: f64 = 0.0;
    for &x in v.iter() {
        sum += x;
    }
    sum
}

fn smoothen(v: &mut [f64]) {
    let length = v.len();
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j: usize = 0;
        while (j as c_int) < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / (N_SMOOTH as f64);
    }
}

fn differentiate(v: &mut [f64]) {
    let length = v.len();
    if length == 0 {
        return;
    }
    for i in 0..(length - 1) {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(dest: &mut [f64], source: &[f64]) {
    // memcpy(v, source, length * sizeof(*v));
    dest.copy_from_slice(source);
    smoothen(dest);
    differentiate(dest);
    smoothen(dest);
}

/// int match(float_t *test, float_t *reference, int bins, double threshold);
///
/// `float_t` in match.h is `double`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut f64,
    reference: *mut f64,
    bins: c_int,
    threshold: f64,
) -> c_int {
    // C VLAs: float_t t[bins], r[bins];
    let bins_usize: usize = if bins < 0 { 0 } else { bins as usize };

    // Build slices over the input pointers.
    let test_slice: &[f64] = unsafe { std::slice::from_raw_parts(test, bins_usize) };
    let reference_slice: &[f64] = unsafe { std::slice::from_raw_parts(reference, bins_usize) };

    // First validation/check: if(total(test, bins) < threshold * total(reference, bins)) return 0;
    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }

    // Local working buffers (mirror the C VLAs t[bins], r[bins]).
    let mut t: Vec<f64> = vec![0.0; bins_usize];
    let mut r: Vec<f64> = vec![0.0; bins_usize];

    preprocess(&mut t, test_slice);
    preprocess(&mut r, reference_slice);

    // Now call the buggy spectral_contrast: it interprets these double arrays
    // as float arrays. We replicate the exact behavior by calling the same
    // routine that the FFI symbol exposes.
    //
    // Note: spectral_contrast mutates the first `bins` 4-byte words of t and r
    // in-place when normalizing, just like the C version does.
    let result: f64 = unsafe {
        spectral_contrast_impl(
            t.as_mut_ptr() as *mut f32,
            r.as_mut_ptr() as *mut f32,
            bins,
        )
    };

    if result >= threshold {
        1
    } else {
        0
    }
}

// ---------- spectral_contrast.c ----------
//
// In the C file, `float_t` is `float` (from <math.h>) since match.h was not
// included. We faithfully reproduce that here using f32.

fn dot_product_f32(a: &[f32], b: &[f32]) -> f64 {
    let mut sum: f64 = 0.0;
    let length = a.len();
    for i in 0..length {
        // C: sum += a[i] * b[i];
        // a[i] and b[i] are float; the multiplication is done in float, then
        // promoted to double (per usual arithmetic conversions for `double sum`).
        let prod_f32: f32 = a[i] * b[i];
        sum += prod_f32 as f64;
    }
    sum
}

fn normalize_f32(v: &mut [f32]) {
    // double magnitude = sqrt(dot_product(v, v, length));
    let magnitude: f64 = dot_product_f32(v, v).sqrt();
    let length = v.len();
    for i in 0..length {
        // C: v[i] /= magnitude;
        // v[i] is float, magnitude is double. Per C: v[i] is promoted to
        // double, divided as double, then converted back to float on store.
        let promoted: f64 = v[i] as f64;
        let result_f64: f64 = promoted / magnitude;
        v[i] = result_f64 as f32;
    }
}

/// Internal implementation, callable from match().
unsafe fn spectral_contrast_impl(a: *mut f32, b: *mut f32, length: c_int) -> f64 {
    let len_usize: usize = if length < 0 { 0 } else { length as usize };
    let a_slice: &mut [f32] = unsafe { std::slice::from_raw_parts_mut(a, len_usize) };
    let b_slice: &mut [f32] = unsafe { std::slice::from_raw_parts_mut(b, len_usize) };
    normalize_f32(a_slice);
    normalize_f32(b_slice);
    dot_product_f32(a_slice, b_slice)
}

/// double spectral_contrast(float_t *a, float_t *b, int length);
///
/// Exposed C ABI symbol. Header declares pointers as `double *` (because
/// match.h's float_t is double), but the C implementation actually treats
/// them as `float *` because spectral_contrast.c does not include match.h
/// and float_t in <math.h> is float. The pointer ABI is the same regardless
/// of pointee type, so the library reinterprets the caller's memory as
/// float. We declare the Rust signature with *mut f32 to match the C source's
/// implementation behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut f32,
    b: *mut f32,
    length: c_int,
) -> f64 {
    unsafe { spectral_contrast_impl(a, b, length) }
}
