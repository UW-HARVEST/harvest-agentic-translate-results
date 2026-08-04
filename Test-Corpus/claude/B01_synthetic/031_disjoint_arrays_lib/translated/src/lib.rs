// Translated from c_src/src/driver.c

use std::ffi::c_char;
use std::os::raw::c_int;

unsafe extern "C" {
    fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[c_int], len: usize) -> c_int {
    if len == 0 {
        return 0;
    }
    // Mirror the C code: declare VLAs, set out[0] = 0, fill ones/zeros.
    let mut out = vec![0 as c_int; len];
    let mut ones = vec![0 as c_int; len];
    let mut zeros = vec![0 as c_int; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let mut data: [c_int; 100] = [0; 100];
    let mut i: usize = 0;
    let mut cursor = input;

    while i < 100 {
        let mut nb: usize = 0;
        let fmt = b"%d%zn\0".as_ptr() as *const c_char;
        let ret = unsafe {
            sscanf(
                cursor,
                fmt,
                &mut data[i] as *mut c_int,
                &mut nb as *mut usize,
            )
        };
        if ret != 1 {
            break;
        }
        cursor = unsafe { cursor.add(nb) };
        i += 1;
    }

    let result = call_fma(&data, i);
    let print_fmt = b"%d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(print_fmt, result);
    }
}
