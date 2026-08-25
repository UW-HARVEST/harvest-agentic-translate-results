use std::ffi::{c_char, c_int};

unsafe fn crash_like_null_access() -> ! {
    (1_usize as *mut u8).write_volatile(0);
    std::process::abort();
}

#[no_mangle]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    if len > 0 && (out.is_null() || mul1.is_null() || mul2.is_null() || add.is_null()) {
        crash_like_null_access();
    }

    for i in 0..len {
        let index = i as usize;
        let value = (*mul1.add(index))
            .wrapping_mul(*mul2.add(index))
            .wrapping_add(*add.add(index));
        *out.add(index) = value;
    }
}

#[no_mangle]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    if len > 0 && data.is_null() {
        crash_like_null_access();
    }

    let len = len as usize;
    let mut out = vec![0_i32; len];
    let ones = vec![1_i32; len];
    let zeros = vec![0_i32; len];
    fma_array(
        out.as_mut_ptr(),
        ones.as_ptr(),
        data,
        zeros.as_ptr(),
        len as c_int,
    );
    out[len - 1]
}

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    let mut data = [0_i32; 100];
    let mut len = 0;

    while len < data.len() {
        if scanf(c"%d".as_ptr(), &mut data[len]) != 1 {
            break;
        }
        len += 1;
    }

    let result = call_fma(data.as_ptr(), len as c_int);
    printf(c"%d\n".as_ptr(), result);
    0
}
