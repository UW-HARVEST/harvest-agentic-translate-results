use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;

unsafe extern "C" {
    fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
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
    let n = len as isize;
    for i in 0..n {
        unsafe {
            let m1 = *mul1.offset(i);
            let m2 = *mul2.offset(i);
            let a = *add.offset(i);
            // Reproduce C's signed integer multiply/add with wrapping semantics
            // (the C source relies on default int arithmetic, which is UB on overflow,
            // but in practice wraps on x86_64; use wrapping ops to be deterministic).
            let prod = m1.wrapping_mul(m2);
            let sum = prod.wrapping_add(a);
            *out.offset(i) = sum;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    let n = len as usize;
    let mut out: Vec<c_int> = vec![0; n];
    let mut ones: Vec<c_int> = vec![0; n];
    let mut zeros: Vec<c_int> = vec![0; n];

    out[0] = 0;
    for i in 0..n {
        ones[i] = 1;
        zeros[i] = 0;
    }

    unsafe {
        fma_array(
            out.as_mut_ptr(),
            ones.as_ptr(),
            data,
            zeros.as_ptr(),
            len,
        );
    }
    out[n - 1]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let mut data: [c_int; 100] = [0; 100];
    let mut i: c_int = 0;
    let mut cursor: *const c_char = input;

    let fmt = b"%d%zn\0";
    let print_fmt = b"%d\n\0";

    while i < 100 {
        let mut nb: usize = 0;
        let r = unsafe {
            sscanf(
                cursor,
                fmt.as_ptr() as *const c_char,
                &mut data[i as usize] as *mut c_int,
                &mut nb as *mut usize,
            )
        };
        if r != 1 {
            break;
        }
        cursor = unsafe { cursor.add(nb) };
        i += 1;
    }

    let result = unsafe { call_fma(data.as_ptr(), i) };
    unsafe {
        printf(print_fmt.as_ptr() as *const c_char, result);
    }
    // Suppress unused warning for c_void import in case of macro changes
    let _ = std::ptr::null::<c_void>();
}
