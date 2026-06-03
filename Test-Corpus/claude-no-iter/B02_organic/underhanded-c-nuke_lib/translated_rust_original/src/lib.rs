use std::ffi::c_int;
use std::os::raw::c_double;

/// Size of smoothing kernel (matches `#define N_SMOOTH 16` in match.h)
const N_SMOOTH: usize = 16;

/// `typedef double float_t` in match.h
type FloatT = c_double;

fn total(v: &[FloatT]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..v.len() {
        sum += v[i];
    }
    sum
}

fn smoothen(v: &mut [FloatT]) {
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

fn differentiate(v: &mut [FloatT]) {
    let length = v.len();
    if length == 0 {
        return;
    }
    for i in 0..(length - 1) {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(dest: &mut [FloatT], source: &[FloatT]) {
    // memcpy(v, source, length * sizeof(*v));
    dest.copy_from_slice(source);
    smoothen(dest);
    differentiate(dest);
    smoothen(dest);
}

fn dot_product(a: &[FloatT], b: &[FloatT]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

fn normalize(v: &mut [FloatT]) {
    let magnitude = dot_product(v, v).sqrt();
    for i in 0..v.len() {
        v[i] /= magnitude;
    }
}

/// Public C function `spectral_contrast`.
///
/// Original signature:
/// `double spectral_contrast(float_t *a, float_t *b, int length);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut FloatT,
    b: *mut FloatT,
    length: c_int,
) -> c_double {
    let len = length as usize;
    let a_slice = std::slice::from_raw_parts_mut(a, len);
    let b_slice = std::slice::from_raw_parts_mut(b, len);
    normalize(a_slice);
    normalize(b_slice);
    dot_product(a_slice, b_slice)
}

/// Public C function `match`.
///
/// Original signature:
/// `int match(float_t *test, float_t *reference, int bins, double threshold);`
///
/// `match` is a reserved keyword in Rust; use a raw identifier so the
/// linker symbol is exactly `match`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut FloatT,
    reference: *mut FloatT,
    bins: c_int,
    threshold: c_double,
) -> c_int {
    let len = bins as usize;
    let test_slice = std::slice::from_raw_parts_mut(test, len);
    let reference_slice = std::slice::from_raw_parts_mut(reference, len);

    // Reproduce: float_t t[bins], r[bins]; (VLAs in C)
    let mut t: Vec<FloatT> = vec![0.0; len];
    let mut r: Vec<FloatT> = vec![0.0; len];

    // if(total(test, bins) < threshold * total(reference, bins)) return 0;
    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }

    preprocess(&mut t, test_slice);
    preprocess(&mut r, reference_slice);

    // return spectral_contrast(t, r, bins) >= threshold;
    // The C code calls spectral_contrast on local arrays t and r.
    let result = {
        normalize(&mut t);
        normalize(&mut r);
        dot_product(&t, &r)
    };

    if result >= threshold {
        1
    } else {
        0
    }
}
