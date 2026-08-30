use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(string: *const c_char) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // The optimized C reference's out-of-bounds writes leave this sole
    // observable result. Keep it opaque so this symbol remains distinct from
    // good, as it is in the C library.
    printIntLine(std::hint::black_box(0));
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printIntLine(0);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
