use std::ffi::c_int;
use std::slice;

const N_SMOOTH: usize = 16;

type FloatT = f64;

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
    let length = v.len();
    for i in 0..length {
        v[i] /= magnitude;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut FloatT,
    b: *mut FloatT,
    length: c_int,
) -> f64 {
    let len = length as usize;
    let a_slice = unsafe { slice::from_raw_parts_mut(a, len) };
    let b_slice = unsafe { slice::from_raw_parts_mut(b, len) };
    normalize(a_slice);
    normalize(b_slice);
    dot_product(a_slice, b_slice)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut FloatT,
    reference: *mut FloatT,
    bins: c_int,
    threshold: f64,
) -> c_int {
    let bins_usize = bins as usize;
    let test_slice = unsafe { slice::from_raw_parts(test, bins_usize) };
    let reference_slice = unsafe { slice::from_raw_parts(reference, bins_usize) };

    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }

    let mut t: Vec<FloatT> = vec![0.0; bins_usize];
    let mut r: Vec<FloatT> = vec![0.0; bins_usize];

    preprocess(&mut t, test_slice);
    preprocess(&mut r, reference_slice);

    let sc = unsafe { spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins) };
    if sc >= threshold { 1 } else { 0 }
}
