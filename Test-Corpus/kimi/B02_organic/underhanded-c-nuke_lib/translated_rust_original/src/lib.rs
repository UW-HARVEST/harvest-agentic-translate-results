use std::os::raw::{c_double, c_int};

pub type FloatT = c_double;

const N_SMOOTH: usize = 16;

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
        v[i] = sum / N_SMOOTH as c_double;
    }
}

fn differentiate(v: &mut [FloatT]) {
    let length = v.len();
    for i in 0..length.saturating_sub(1) {
        v[i] = v[i + 1] - v[i];
    }
    if length > 0 {
        v[length - 1] = 0.0;
    }
}

fn preprocess(v: &mut [FloatT], source: &[FloatT]) {
    v.copy_from_slice(source);
    smoothen(v);
    differentiate(v);
    smoothen(v);
}

fn dot_product(a: &[FloatT], b: &[FloatT]) -> c_double {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn normalize(v: &mut [FloatT]) {
    let magnitude = dot_product(v, v).sqrt();
    if magnitude > 0.0 {
        for elem in v.iter_mut() {
            *elem /= magnitude;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn spectral_contrast(a: *mut FloatT, b: *mut FloatT, length: c_int) -> c_double {
    let len = length as usize;
    let a_slice = unsafe { std::slice::from_raw_parts_mut(a, len) };
    let b_slice = unsafe { std::slice::from_raw_parts_mut(b, len) };
    normalize(a_slice);
    normalize(b_slice);
    dot_product(a_slice, b_slice)
}

#[unsafe(no_mangle)]
pub extern "C" fn match(test: *mut FloatT, reference: *mut FloatT, bins: c_int, threshold: c_double) -> c_int {
    let bins_usize = bins as usize;
    let test_slice = unsafe { std::slice::from_raw_parts(test, bins_usize) };
    let reference_slice = unsafe { std::slice::from_raw_parts(reference, bins_usize) };
    
    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }
    
    let mut t = vec![0.0; bins_usize];
    let mut r = vec![0.0; bins_usize];
    
    preprocess(&mut t, test_slice);
    preprocess(&mut r, reference_slice);
    
    if spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins) >= threshold {
        1
    } else {
        0
    }
}
