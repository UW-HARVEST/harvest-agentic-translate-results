use std::ffi::{c_char, c_int};

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
    for i in 0..len {
        let offset = i as isize;
        unsafe {
            *out.offset(offset) = (*mul1.offset(offset))
                .wrapping_mul(*mul2.offset(offset))
                .wrapping_add(*add.offset(offset));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }

    let len_usize = len as usize;
    let mut out = vec![0 as c_int; len_usize];
    let ones = vec![1 as c_int; len_usize];
    let zeros = vec![0 as c_int; len_usize];

    unsafe {
        fma_array(
            out.as_mut_ptr(),
            ones.as_ptr(),
            data,
            zeros.as_ptr(),
            len,
        );
    }

    out[len_usize - 1]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(mut in_: *const c_char) {
    let mut data = [0 as c_int; 100];
    let mut i = 0usize;

    while i < 100 {
        let mut nb = 0usize;
        let parsed = unsafe {
            sscanf(
                in_,
                c"%d%zn".as_ptr(),
                data.as_mut_ptr().add(i),
                &mut nb as *mut usize,
            )
        };
        if parsed != 1 {
            break;
        }
        unsafe {
            in_ = in_.add(nb);
        }
        i += 1;
    }

    let result = unsafe { call_fma(data.as_ptr(), i as c_int) };
    unsafe {
        printf(c"%d\n".as_ptr(), result);
    }
}
