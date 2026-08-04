// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len {
        let i_us = i as isize;
        unsafe {
            *out.offset(i_us) =
                (*mul1.offset(i_us)).wrapping_mul(*mul2.offset(i_us)).wrapping_add(*add.offset(i_us));
        }
    }
}

unsafe fn inner(out: *mut c_int, len: c_int) {
    unsafe {
        fma_array(out, out, out, out, len);
        let fmt = b"%d\n\0".as_ptr() as *const c_char;
        for i in 0..len {
            printf(fmt, *out.offset(i as isize));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // C code: int out[len]; memcpy(out, data, len * sizeof(int));
    // VLA on stack — emulate with a Vec of length `len`.
    let n = if len < 0 { 0 } else { len as usize };
    let mut out: Vec<c_int> = vec![0; n];
    unsafe {
        if n > 0 {
            std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), n);
        }
        inner(out.as_mut_ptr(), len);
    }
}
