use std::ffi::{c_double, c_float, c_int};
use std::mem::MaybeUninit;

const N_SMOOTH: c_int = 16;

#[cfg(target_arch = "x86_64")]
unsafe fn raise_sigsegv() -> ! {
    unsafe {
        std::arch::asm!(
            "mov byte ptr [{address}], 0",
            address = in(reg) 0_usize,
            options(noreturn, nostack)
        );
    }
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn raise_sigsegv() -> ! {
    unsafe extern "C" {
        fn raise(signal: c_int) -> c_int;
    }
    unsafe {
        raise(11);
        std::hint::unreachable_unchecked();
    }
}

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

#[cfg(target_arch = "x86_64")]
fn multiply_and_accumulate(sum: c_double, a: c_float, b: c_float) -> c_double {
    let mut product = b;
    unsafe {
        std::arch::asm!(
            "mulss {product}, {a}",
            product = inout(xmm_reg) product,
            a = in(xmm_reg) a,
            options(pure, nomem, nostack)
        );
    }

    let mut result = product as c_double;
    unsafe {
        std::arch::asm!(
            "addsd {result}, {sum}",
            result = inout(xmm_reg) result,
            sum = in(xmm_reg) sum,
            options(pure, nomem, nostack)
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
fn multiply_and_accumulate(sum: c_double, a: c_float, b: c_float) -> c_double {
    sum + (a * b) as c_double
}

unsafe fn dot_product(a: *mut c_float, b: *mut c_float, length: c_int) -> c_double {
    let mut sum = 0.0;
    let mut i = 0;
    while i < length {
        let a_value = unsafe { *a.offset(i as isize) };
        let b_value = unsafe { *b.offset(i as isize) };
        sum = multiply_and_accumulate(sum, a_value, b_value);
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
    if bins <= 0 || bins == c_int::MAX {
        unsafe {
            raise_sigsegv();
        }
    }

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
