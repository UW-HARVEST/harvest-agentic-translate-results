// IMPORTANT BUG REPRODUCTION:
// The C source `spectral_contrast.c` only includes <math.h>, NOT "match.h".
// On glibc, <math.h> provides a typedef `float_t` (commonly = `float`, 4 bytes).
// In `match.h`, `float_t` is typedef'd to `double` (8 bytes).
// match.c includes match.h so its `float_t` is `double`. spectral_contrast.c
// includes only <math.h> so its `float_t` is `float`. The two files therefore
// disagree on the element type of the arrays passed across the boundary.
// At the ABI level this is fine (they're just pointers) but internally
// spectral_contrast reads/writes the memory as a `float[]` array of the given
// length, while match's local buffers `t[bins]`, `r[bins]` are actually
// `double[]`. This is undefined behavior in C but produces deterministic
// output on the target machine, which we must reproduce byte-for-byte.

use std::ffi::c_int;
use std::slice;

const N_SMOOTH: usize = 16;

// In match.c, `float_t` resolves to `double`.
#[allow(non_camel_case_types)]
type float_t_match = f64;

// In spectral_contrast.c, `float_t` resolves to `float` (from <math.h>).
#[allow(non_camel_case_types)]
type float_t_sc = f32;

// ---- helpers used by `match` (operate on f64) ----

fn total(v: &[float_t_match]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..v.len() {
        sum += v[i];
    }
    sum
}

fn smoothen(v: &mut [float_t_match]) {
    let length = v.len();
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j: usize = 0;
        while j < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / N_SMOOTH as f64;
    }
}

fn differentiate(v: &mut [float_t_match]) {
    let length = v.len();
    if length == 0 {
        return;
    }
    for i in 0..(length - 1) {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(v: &mut [float_t_match], source: &[float_t_match]) {
    v.copy_from_slice(source);
    smoothen(v);
    differentiate(v);
    smoothen(v);
}

// ---- helpers used by `spectral_contrast` (operate on f32) ----
// Note: dot_product accumulates into a `double` (f64).

fn sc_dot_product(a: &[float_t_sc], b: &[float_t_sc]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..a.len() {
        // C: sum += a[i] * b[i]; with a/b being float, the multiplication is
        // a float multiplication (per the usual arithmetic conversions on the
        // float operands), then promoted to double for the addition.
        let prod: f32 = a[i] * b[i];
        sum += prod as f64;
    }
    sum
}

fn sc_normalize(v: &mut [float_t_sc]) {
    // C: double magnitude = sqrt(dot_product(v, v, length));
    let magnitude: f64 = sc_dot_product(v, v).sqrt();
    for i in 0..v.len() {
        // C: v[i] /= magnitude;
        // v[i] is float; magnitude is double. v[i] is promoted to double,
        // divided, then assigned back to float (truncating to f32).
        let promoted: f64 = v[i] as f64;
        v[i] = (promoted / magnitude) as f32;
    }
}

fn spectral_contrast_impl_f32(a: &mut [float_t_sc], b: &mut [float_t_sc]) -> f64 {
    sc_normalize(a);
    sc_normalize(b);
    sc_dot_product(a, b)
}

// ---- Public exported FFI symbols ----

/// `spectral_contrast` from spectral_contrast.c. The pointers are formally
/// `float_t *` (which the implementation file resolves to `float *`). Length
/// is the number of `float` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut f32,
    b: *mut f32,
    length: c_int,
) -> f64 {
    let len = length as usize;
    let a_slice = slice::from_raw_parts_mut(a, len);
    let b_slice = slice::from_raw_parts_mut(b, len);
    spectral_contrast_impl_f32(a_slice, b_slice)
}

/// `match` from match.c. The pointers are `double *` (because match.h's
/// `float_t` is `double`). `bins` is the number of `double` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut f64,
    reference: *mut f64,
    bins: c_int,
    threshold: f64,
) -> c_int {
    let len = bins as usize;
    let test_slice = slice::from_raw_parts(test, len);
    let reference_slice = slice::from_raw_parts(reference, len);

    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }

    // Local buffers t[bins], r[bins] in match.c are `double` arrays.
    let mut t: Vec<f64> = vec![0.0; len];
    let mut r: Vec<f64> = vec![0.0; len];
    preprocess(&mut t, test_slice);
    preprocess(&mut r, reference_slice);

    // Now match.c calls spectral_contrast(t, r, bins). At the C ABI this is
    // just two pointers and an int. spectral_contrast.c reinterprets the
    // memory as `float` arrays of length `bins`. We must do the same: hand
    // it pointers to the double-typed buffers but treat each element as a
    // 4-byte float internally.
    let t_as_f32: &mut [f32] = std::slice::from_raw_parts_mut(t.as_mut_ptr() as *mut f32, len);
    let r_as_f32: &mut [f32] = std::slice::from_raw_parts_mut(r.as_mut_ptr() as *mut f32, len);
    let result: f64 = spectral_contrast_impl_f32(t_as_f32, r_as_f32);

    if result >= threshold {
        1
    } else {
        0
    }
}
