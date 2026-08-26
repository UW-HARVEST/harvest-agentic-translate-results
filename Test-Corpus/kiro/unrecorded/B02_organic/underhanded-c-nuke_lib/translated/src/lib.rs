use std::os::raw::{c_double, c_int};

const N_SMOOTH: usize = 16;

type FloatT = c_double;

fn total(v: &[FloatT]) -> c_double {
    v.iter().sum()
}

fn smoothen(v: &mut [FloatT]) {
    let length = v.len();
    for i in 0..length {
        let mut sum = 0.0;
        for j in 0..N_SMOOTH {
            if i + j >= length {
                break;
            }
            sum += v[i + j];
        }
        v[i] = sum / N_SMOOTH as f64;
    }
}

fn differentiate(v: &mut [FloatT]) {
    let length = v.len();
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(source: &[FloatT], length: usize) -> Vec<FloatT> {
    let mut v = source[..length].to_vec();
    smoothen(&mut v);
    differentiate(&mut v);
    smoothen(&mut v);
    v
}

fn dot_product(a: &[FloatT], b: &[FloatT]) -> c_double {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn normalize(v: &mut [FloatT]) {
    let magnitude = dot_product(v, v).sqrt();
    for x in v.iter_mut() {
        *x /= magnitude;
    }
}

fn spectral_contrast_impl(a: &mut [FloatT], b: &mut [FloatT]) -> c_double {
    normalize(a);
    normalize(b);
    dot_product(a, b)
}

#[unsafe(export_name = "match")]
pub extern "C" fn match_(test: *mut FloatT, reference: *mut FloatT, bins: c_int, threshold: c_double) -> c_int {
    let bins = bins as usize;
    let test_slice = unsafe { std::slice::from_raw_parts(test, bins) };
    let ref_slice = unsafe { std::slice::from_raw_parts(reference, bins) };
    if total(test_slice) < threshold * total(ref_slice) {
        return 0;
    }
    let mut t = preprocess(test_slice, bins);
    let mut r = preprocess(ref_slice, bins);
    (spectral_contrast_impl(&mut t, &mut r) >= threshold) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn spectral_contrast(a: *mut FloatT, b: *mut FloatT, length: c_int) -> c_double {
    let length = length as usize;
    let a_slice = unsafe { std::slice::from_raw_parts_mut(a, length) };
    let b_slice = unsafe { std::slice::from_raw_parts_mut(b, length) };
    normalize(a_slice);
    normalize(b_slice);
    dot_product(a_slice, b_slice)
}
