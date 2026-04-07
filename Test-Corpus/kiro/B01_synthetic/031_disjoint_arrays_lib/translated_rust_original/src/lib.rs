use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[c_int], len: usize) -> c_int {
    if len == 0 {
        return 0;
    }
    let mut out: Vec<c_int> = vec![0; len];
    let ones: Vec<c_int> = vec![1; len];
    let zeros: Vec<c_int> = vec![0; len];

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut input: *const c_char) {
    let mut data: [c_int; 100] = [0; 100];
    let mut i = 0usize;
    while i < 100 {
        let mut val: c_int = 0;
        let mut nb: isize = 0;
        let ret = unsafe {
            sscanf(
                input,
                b"%d%zn\0".as_ptr() as *const c_char,
                &mut val as *mut c_int,
                &mut nb as *mut isize,
            )
        };
        if ret != 1 {
            break;
        }
        data[i] = val;
        input = unsafe { input.offset(nb) };
        i += 1;
    }

    let result = call_fma(&data, i);
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, result);
    }
}
