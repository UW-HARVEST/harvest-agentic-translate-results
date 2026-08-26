use std::ffi::{c_double, c_float, c_int};
use std::mem::MaybeUninit;

const N_SMOOTH: c_int = 16;

unsafe fn total(v: *mut c_double, length: c_int) -> c_double {
    let mut sum = 0.0;
    let mut i = 0;
    while i < length {
        sum += unsafe { *v.offset(i as isize) };
        i += 1;
    }
    sum
}

unsafe fn smoothen(v: *mut c_double, length: c_int) {
    let mut i = 0;
    while i < length {
        let mut sum = 0.0;
        let mut j = 0;
        while j < N_SMOOTH && i + j < length {
            sum += unsafe { *v.offset((i + j) as isize) };
            j += 1;
        }
        unsafe {
            *v.offset(i as isize) = sum / N_SMOOTH as c_double;
        }
        i += 1;
    }
}

unsafe fn differentiate(v: *mut c_double, length: c_int) {
    let mut i = 0;
    while i < length - 1 {
        unsafe {
            *v.offset(i as isize) = *v.offset((i + 1) as isize) - *v.offset(i as isize);
        }
        i += 1;
    }
    unsafe {
        *v.offset((length - 1) as isize) = 0.0;
    }
}

unsafe fn preprocess(v: *mut c_double, source: *mut c_double, length: c_int) {
    unsafe {
        std::ptr::copy_nonoverlapping(source, v, length as usize);
        smoothen(v, length);
        differentiate(v, length);
        smoothen(v, length);
    }
}

unsafe fn dot_product(a: *mut c_float, b: *mut c_float, length: c_int) -> c_double {
    let mut sum = 0.0;
    let mut i = 0;
    while i < length {
        sum += unsafe { (*a.offset(i as isize) * *b.offset(i as isize)) as c_double };
        i += 1;
    }
    sum
}

unsafe fn normalize(v: *mut c_float, length: c_int) {
    let magnitude = unsafe { dot_product(v, v, length) }.sqrt();
    let mut i = 0;
    while i < length {
        unsafe {
            *v.offset(i as isize) = (*v.offset(i as isize) as c_double / magnitude) as c_float;
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
    unsafe {
        let a = a.cast::<c_float>();
        let b = b.cast::<c_float>();
        normalize(a, length);
        normalize(b, length);
        dot_product(a, b, length)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut c_double,
    reference: *mut c_double,
    bins: c_int,
    threshold: c_double,
) -> c_int {
    let mut t = vec![MaybeUninit::<c_double>::uninit(); bins as usize];
    let mut r = vec![MaybeUninit::<c_double>::uninit(); bins as usize];

    if unsafe { total(test, bins) } < threshold * unsafe { total(reference, bins) } {
        return 0;
    }

    unsafe {
        let t = t.as_mut_ptr().cast::<c_double>();
        let r = r.as_mut_ptr().cast::<c_double>();
        preprocess(t, test, bins);
        preprocess(r, reference, bins);
        (spectral_contrast(t, r, bins) >= threshold) as c_int
    }
}
