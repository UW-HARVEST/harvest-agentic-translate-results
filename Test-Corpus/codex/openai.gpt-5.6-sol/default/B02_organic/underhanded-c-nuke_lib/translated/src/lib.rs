use std::ffi::{c_double, c_int};
use std::ptr;

const N_SMOOTH: usize = 16;

unsafe fn total(v: *const c_double, length: c_int) -> c_double {
    let mut sum = 0.0;
    let mut i = 0;
    while i < length {
        sum += unsafe { *v.offset(i as isize) };
        i += 1;
    }
    sum
}

fn smoothen(v: &mut [c_double]) {
    for i in 0..v.len() {
        let mut sum = 0.0;
        let mut j = 0;
        while j < N_SMOOTH && i + j < v.len() {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / N_SMOOTH as c_double;
    }
}

fn differentiate(v: &mut [c_double]) {
    for i in 0..v.len() - 1 {
        v[i] = v[i + 1] - v[i];
    }
    let last = v.len() - 1;
    v[last] = 0.0;
}

unsafe fn preprocess(v: &mut [c_double], source: *const c_double) {
    unsafe {
        ptr::copy_nonoverlapping(source, v.as_mut_ptr(), v.len());
    }
    smoothen(v);
    differentiate(v);
    smoothen(v);
}

unsafe fn dot_product_f32(a: *const f32, b: *const f32, length: c_int) -> c_double {
    let mut sum = 0.0;
    let mut i = 0;
    while i < length {
        let product = unsafe { *a.offset(i as isize) * *b.offset(i as isize) };
        sum += product as c_double;
        i += 1;
    }
    sum
}

unsafe fn normalize_f32(v: *mut f32, length: c_int) {
    let magnitude = unsafe { dot_product_f32(v, v, length) }.sqrt();
    let mut i = 0;
    while i < length {
        let value = unsafe { *v.offset(i as isize) } as c_double / magnitude;
        unsafe {
            *v.offset(i as isize) = value as f32;
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut c_double,
    b: *mut c_double,
    length: c_int,
) -> c_double {
    let a = a.cast::<f32>();
    let b = b.cast::<f32>();
    unsafe {
        normalize_f32(a, length);
        normalize_f32(b, length);
        dot_product_f32(a, b, length)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut c_double,
    reference: *mut c_double,
    bins: c_int,
    threshold: c_double,
) -> c_int {
    if unsafe { total(test, bins) } < threshold * unsafe { total(reference, bins) } {
        return 0;
    }

    let length = bins as usize;
    let mut t = vec![0.0; length];
    let mut r = vec![0.0; length];
    unsafe {
        preprocess(&mut t, test);
        preprocess(&mut r, reference);
    }

    (unsafe { spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins) } >= threshold) as c_int
}
