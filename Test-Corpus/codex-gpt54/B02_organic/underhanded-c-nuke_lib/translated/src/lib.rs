use std::ffi::{c_double, c_int};
use std::slice;

const N_SMOOTH: usize = 16;

fn total(v: &[c_double]) -> c_double {
    let mut sum = 0.0;
    let mut i = 0;
    while i < v.len() {
        sum += v[i];
        i += 1;
    }
    sum
}

fn smoothen(v: &mut [c_double]) {
    let length = v.len();
    let mut i = 0;
    while i < length {
        let mut sum = 0.0;
        let mut j = 0;
        while j < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / N_SMOOTH as c_double;
        i += 1;
    }
}

fn differentiate(v: &mut [c_double]) {
    let length = v.len();
    let mut i = 0;
    while i + 1 < length {
        v[i] = v[i + 1] - v[i];
        i += 1;
    }
    v[length - 1] = 0.0;
}

fn preprocess(v: &mut [c_double], source: &[c_double]) {
    v.copy_from_slice(source);
    smoothen(v);
    differentiate(v);
    smoothen(v);
}

fn dot_product(a: &[c_double], b: &[c_double]) -> c_double {
    let mut sum = 0.0;
    let mut i = 0;
    while i < a.len() {
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}

fn normalize(v: &mut [c_double]) {
    let magnitude = dot_product(v, v).sqrt();
    let mut i = 0;
    while i < v.len() {
        v[i] /= magnitude;
        i += 1;
    }
}

unsafe fn slice_from_raw_parts_mut<'a>(ptr: *mut c_double, len: usize) -> &'a mut [c_double] {
    unsafe { slice::from_raw_parts_mut(ptr, len) }
}

unsafe fn slice_from_raw_parts<'a>(ptr: *const c_double, len: usize) -> &'a [c_double] {
    unsafe { slice::from_raw_parts(ptr, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut c_double,
    b: *mut c_double,
    length: c_int,
) -> c_double {
    let length = length as usize;
    let a = unsafe { slice_from_raw_parts_mut(a, length) };
    let b = unsafe { slice_from_raw_parts_mut(b, length) };
    normalize(a);
    normalize(b);
    dot_product(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut c_double,
    reference: *mut c_double,
    bins: c_int,
    threshold: c_double,
) -> c_int {
    let bins = bins as usize;
    let test_slice = unsafe { slice_from_raw_parts(test.cast_const(), bins) };
    let reference_slice = unsafe { slice_from_raw_parts(reference.cast_const(), bins) };

    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }

    let mut t = vec![0.0; bins];
    let mut r = vec![0.0; bins];
    preprocess(&mut t, test_slice);
    preprocess(&mut r, reference_slice);

    if unsafe { spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins as c_int) } >= threshold {
        1
    } else {
        0
    }
}
