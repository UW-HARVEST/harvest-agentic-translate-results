use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const INT_LINE_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn printIntPtrLine(int_number: *const c_int) {
    unsafe {
        printf(INT_LINE_FORMAT.as_ptr().cast(), *int_number);
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn bad() {
    let data = std::hint::black_box(std::ptr::null());
    unsafe {
        printIntPtrLine(data);
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn good() {
    let data: c_int = 5;
    let data_addr = &data;
    unsafe {
        printIntPtrLine(data_addr);
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        unsafe {
            good();
        }
    } else {
        unsafe {
            bad();
        }
    }
}
