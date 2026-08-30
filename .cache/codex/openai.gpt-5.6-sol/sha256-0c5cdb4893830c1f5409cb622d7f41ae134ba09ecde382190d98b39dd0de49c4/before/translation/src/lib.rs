use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn sscanf(input: *const c_char, format: *const c_char, ...) -> c_int;
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
    for i in 0..len as isize {
        // C permits unchecked pointer arithmetic and wraps here on the target build.
        unsafe {
            *out.offset(i) = (*mul1.offset(i))
                .wrapping_mul(*mul2.offset(i))
                .wrapping_add(*add.offset(i));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }

    let len = len as usize;
    let mut out = vec![0; len];
    let ones = vec![1; len];
    let zeros = vec![0; len];

    unsafe {
        fma_array(out.as_mut_ptr(), ones.as_ptr(), data, zeros.as_ptr(), len as c_int);
        *out.get_unchecked(len - 1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(mut input: *const c_char) {
    const SCAN_FORMAT: &[u8] = b"%d%zn\0";
    const PRINT_FORMAT: &[u8] = b"%d\n\0";

    let mut data = [0 as c_int; 100];
    let mut count = 0;

    while count < data.len() {
        let mut consumed = 0usize;
        let scanned = unsafe {
            sscanf(
                input,
                SCAN_FORMAT.as_ptr().cast(),
                &mut data[count] as *mut c_int,
                &mut consumed as *mut usize,
            )
        };
        if scanned != 1 {
            break;
        }
        input = unsafe { input.add(consumed) };
        count += 1;
    }

    let result = unsafe { call_fma(data.as_ptr(), count as c_int) };
    unsafe {
        printf(PRINT_FORMAT.as_ptr().cast(), result);
    }
}
