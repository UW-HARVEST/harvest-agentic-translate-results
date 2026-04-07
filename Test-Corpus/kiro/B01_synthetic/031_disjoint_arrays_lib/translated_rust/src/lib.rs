use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(out: *mut c_int, mul1: *const c_int, mul2: *const c_int, add: *const c_int, len: c_int) {
    for i in 0..len as usize {
        unsafe { *out.add(i) = *mul1.add(i) * (*mul2.add(i)) + *add.add(i) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    let len_u = len as usize;
    let mut out: Vec<c_int> = vec![0; len_u];
    let ones: Vec<c_int> = vec![1; len_u];
    let zeros: Vec<c_int> = vec![0; len_u];

    unsafe { fma_array(out.as_mut_ptr(), ones.as_ptr(), data, zeros.as_ptr(), len) };
    out[len_u - 1]
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

    let result = call_fma(data.as_ptr(), i as c_int);
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, result);
    }
}
