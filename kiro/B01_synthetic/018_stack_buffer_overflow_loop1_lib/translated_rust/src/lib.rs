use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe { printf(b"%d\n\0".as_ptr() as *const c_char, int_number) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // alloca(10) — only 10 bytes, but we write 10 ints (buffer overflow)
    let mut buf = [0u8; 10];
    let data: *mut c_int = buf.as_mut_ptr() as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        unsafe { *data.add(i) = source[i] };
    }
    unsafe { printIntLine(*data) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // alloca(10 * sizeof(int)) — 40 bytes, correct size
    let mut buf = [0u8; 10 * std::mem::size_of::<c_int>()];
    let data: *mut c_int = buf.as_mut_ptr() as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        unsafe { *data.add(i) = source[i] };
    }
    unsafe { printIntLine(*data) };
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
