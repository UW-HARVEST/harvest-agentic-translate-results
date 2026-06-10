// Translation of c_src/src/match.c and c_src/src/spectral_contrast.c
//
// Important type-aliasing note:
//
// `match.h` does:
//     typedef double float_t;
// so within `match.c` (which includes `match.h`), `float_t` is `double`.
//
// `spectral_contrast.c` does NOT include `match.h`. It only includes
// `<math.h>`, where the standard C99 `float_t` typedef is defined.
// On x86-64 Linux glibc (FLT_EVAL_METHOD == 0), `float_t` is `float`
// (verified: sizeof(float_t) == 4).
//
// This means:
//   - `match` is compiled with `float_t` == `double` (8 bytes).
//   - `spectral_contrast` is compiled with `float_t` == `float` (4 bytes).
//
// The two translation units disagree on the type, but the linker only
// matches by symbol name, so when `match` calls `spectral_contrast`,
// it passes a `double*` that `spectral_contrast` treats as `float*`.
// This is technically a bug in the C code, but we must reproduce its
// behaviour byte-identically.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

const N_SMOOTH: usize = 16;

// ---------------------------------------------------------------------------
// match.c — `float_t` == `double` (f64) within this translation unit.
// ---------------------------------------------------------------------------

fn total_match(v: &[f64]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..v.len() {
        sum += v[i];
    }
    sum
}

fn smoothen_match(v: &mut [f64]) {
    let length = v.len();
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j: usize = 0;
        while j < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / (N_SMOOTH as f64);
    }
}

fn differentiate_match(v: &mut [f64]) {
    let length = v.len();
    if length == 0 {
        return;
    }
    for i in 0..(length - 1) {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess_match(v: &mut [f64], source: &[f64]) {
    // memcpy(v, source, length * sizeof(*v));
    v.copy_from_slice(source);
    smoothen_match(v);
    differentiate_match(v);
    smoothen_match(v);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut f64,
    reference: *mut f64,
    bins: c_int,
    threshold: f64,
) -> c_int {
    // Reproduce the C source ordering exactly:
    //
    //     float_t t[bins], r[bins];
    //     if(total(test, bins) < threshold * total(reference, bins)) return 0;
    //     preprocess(t, test, bins);
    //     preprocess(r, reference, bins);
    //     return spectral_contrast(t, r, bins) >= threshold;
    //
    // (Note: the VLA declaration has no observable effect besides
    // reserving stack space; in C the early-return path never reads
    // from t or r, so we can declare the buffers after the check.)

    let n = bins as usize;
    let test_slice = std::slice::from_raw_parts(test, n);
    let reference_slice = std::slice::from_raw_parts(reference, n);

    if total_match(test_slice) < threshold * total_match(reference_slice) {
        return 0;
    }

    let mut t: Vec<f64> = vec![0.0; n];
    let mut r: Vec<f64> = vec![0.0; n];

    preprocess_match(&mut t, test_slice);
    preprocess_match(&mut r, reference_slice);

    // spectral_contrast was compiled with `float_t == float`, so it
    // reinterprets these double buffers as float buffers. Pass the
    // raw pointer cast to *mut f32 to reproduce that aliasing.
    let result = spectral_contrast(t.as_mut_ptr() as *mut f32, r.as_mut_ptr() as *mut f32, bins);

    if result >= threshold {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// spectral_contrast.c — `float_t` == `float` (f32) within this translation
// unit (because it only includes <math.h>, not match.h).
// ---------------------------------------------------------------------------

fn dot_product_sc(a: &[f32], b: &[f32]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..a.len() {
        // C: sum += a[i] * b[i];  with a[i], b[i] of type float.
        // The C expression `a[i] * b[i]` is a float multiplication,
        // then promoted to double when added to `sum`.
        sum += (a[i] * b[i]) as f64;
    }
    sum
}

fn normalize_sc(v: &mut [f32]) {
    // double magnitude = sqrt(dot_product(v, v, length));
    let magnitude: f64 = dot_product_sc(v, v).sqrt();
    let length = v.len();
    for i in 0..length {
        // C: v[i] /= magnitude;
        // v[i] is float, magnitude is double. The division is performed
        // in double (v[i] promoted to double), then the result is
        // converted back to float on assignment.
        v[i] = ((v[i] as f64) / magnitude) as f32;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut f32,
    b: *mut f32,
    length: c_int,
) -> f64 {
    let n = length as usize;
    let a_slice = std::slice::from_raw_parts_mut(a, n);
    let b_slice = std::slice::from_raw_parts_mut(b, n);
    normalize_sc(a_slice);
    normalize_sc(b_slice);
    dot_product_sc(a_slice, b_slice)
}
