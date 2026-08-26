use std::ffi::{c_double, c_float, c_int};
use std::ptr;

const N_SMOOTH: usize = 16;

fn total(v: &[c_double]) -> c_double {
    let mut sum = 0.0;
    for &value in v {
        sum += value;
    }
    sum
}

fn smoothen(v: &mut [c_double]) {
    let length = v.len();
    for i in 0..length {
        let mut sum = 0.0;
        let mut j = 0usize;
        while j < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / N_SMOOTH as c_double;
    }
}

fn differentiate(v: &mut [c_double]) {
    let length = v.len();
    for i in 0..length.saturating_sub(1) {
        v[i] = v[i + 1] - v[i];
    }
    if length != 0 {
        v[length - 1] = 0.0;
    }
}

fn preprocess(source: &[c_double]) -> Vec<c_double> {
    let mut v = vec![0.0; source.len()];
    unsafe {
        ptr::copy_nonoverlapping(source.as_ptr(), v.as_mut_ptr(), source.len());
    }
    smoothen(&mut v);
    differentiate(&mut v);
    smoothen(&mut v);
    v
}

fn dot_product_f32(a: *mut c_float, b: *mut c_float, length: c_int) -> c_double {
    let mut sum = 0.0;
    for i in 0..length {
        unsafe {
            let product = *a.add(i as usize) * *b.add(i as usize);
            sum += product as c_double;
        }
    }
    sum
}

fn normalize_f32(v: *mut c_float, length: c_int) {
    let magnitude = dot_product_f32(v, v, length).sqrt();
    for i in 0..length {
        unsafe {
            let value = *v.add(i as usize) as c_double / magnitude;
            *v.add(i as usize) = value as c_float;
        }
    }
}

fn spectral_contrast_impl(a: *mut c_float, b: *mut c_float, length: c_int) -> c_double {
    normalize_f32(a, length);
    normalize_f32(b, length);
    dot_product_f32(a, b, length)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut c_double,
    reference: *mut c_double,
    bins: c_int,
    threshold: c_double,
) -> c_int {
    let bins_usize = bins as usize;
    let test_slice = unsafe { std::slice::from_raw_parts(test, bins_usize) };
    let reference_slice = unsafe { std::slice::from_raw_parts(reference, bins_usize) };

    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }

    let mut t = preprocess(test_slice);
    let mut r = preprocess(reference_slice);
    let contrast = spectral_contrast_impl(
        t.as_mut_ptr() as *mut c_float,
        r.as_mut_ptr() as *mut c_float,
        bins,
    );

    (contrast >= threshold) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut c_double,
    b: *mut c_double,
    length: c_int,
) -> c_double {
    spectral_contrast_impl(a as *mut c_float, b as *mut c_float, length)
}
