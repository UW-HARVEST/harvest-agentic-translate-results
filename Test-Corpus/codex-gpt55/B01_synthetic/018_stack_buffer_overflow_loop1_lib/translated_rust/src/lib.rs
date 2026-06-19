use std::ffi::{c_char, c_int};
use std::ptr::{read_volatile, write_volatile};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const STR_LINE_FORMAT: &[u8] = b"%s\n\0";
const INT_LINE_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(STR_LINE_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(INT_LINE_FORMAT.as_ptr().cast(), int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let mut storage = [0_u8; 10];
    let data = storage.as_mut_ptr().cast::<c_int>();
    let source = [0 as c_int; 10];

    for (i, value) in source.iter().enumerate() {
        unsafe {
            write_volatile(data.add(i), *value);
        }
    }

    unsafe {
        printIntLine(read_volatile(data));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    #[allow(unused_assignments)]
    let mut data: *mut c_int = std::ptr::null_mut();
    let mut storage = [0 as c_int; 10];
    data = storage.as_mut_ptr();
    let source = [0 as c_int; 10];

    for (i, value) in source.iter().enumerate() {
        unsafe {
            write_volatile(data.add(i), *value);
        }
    }

    unsafe {
        printIntLine(read_volatile(data));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
