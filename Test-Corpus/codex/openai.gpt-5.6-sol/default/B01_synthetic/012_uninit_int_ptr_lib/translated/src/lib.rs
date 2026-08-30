use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(int_number: *const c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), *int_number);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data = std::hint::black_box(std::ptr::null());
    unsafe {
        printIntPtrLine(data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let data: c_int = 5;
    unsafe {
        printIntPtrLine(&data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        unsafe {
            bad();
        }
    }
}
