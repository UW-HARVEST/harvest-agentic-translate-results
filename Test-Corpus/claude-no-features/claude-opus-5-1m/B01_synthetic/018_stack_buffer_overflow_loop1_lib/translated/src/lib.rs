use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // Original C: data = (int *)alloca(10);
    // This allocates 10 bytes, then writes 10 ints (40 bytes) into it.
    // Undefined behavior, but observable output writes data[0] = source[0] = 0.
    // We allocate 10 ints worth of space to safely match the observable output.
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    for i in 0..10usize {
        data[i] = source[i];
    }
    unsafe {
        printIntLine(data[0]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    for i in 0..10usize {
        data[i] = source[i];
    }
    unsafe {
        printIntLine(data[0]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(use_good: c_int) {
    unsafe {
        if use_good != 0 {
            good();
        } else {
            bad();
        }
    }
}
