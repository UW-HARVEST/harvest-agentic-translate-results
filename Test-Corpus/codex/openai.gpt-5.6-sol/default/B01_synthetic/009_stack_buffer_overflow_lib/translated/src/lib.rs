use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const STRING_FORMAT: &[u8] = b"%s\n\0";
const INT_FORMAT: &[u8] = b"%d\n\0";
const NEGATIVE_INDEX_ERROR: &[u8] = b"ERROR: Array index is negative.\0";
const OUT_OF_BOUNDS_ERROR: &[u8] = b"ERROR: Array index is out-of-bounds\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(STRING_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(INT_FORMAT.as_ptr().cast(), int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_int) {
    let mut buffer = [0; 10];

    if data >= 0 {
        unsafe {
            buffer.as_mut_ptr().add(data as usize).write(1);
        }
        for value in buffer {
            printIntLine(value);
        }
    } else {
        printLine(NEGATIVE_INDEX_ERROR.as_ptr().cast());
    }
}

fn good_g2b() {
    let data = 7;
    let mut buffer = [0; 10];

    if data >= 0 {
        buffer[data as usize] = 1;
        for value in buffer {
            printIntLine(value);
        }
    } else {
        printLine(NEGATIVE_INDEX_ERROR.as_ptr().cast());
    }
}

fn good_b2g(data: c_int) {
    let mut buffer = [0; 10];

    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for value in buffer {
            printIntLine(value);
        }
    } else {
        printLine(OUT_OF_BOUNDS_ERROR.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    printLine(c"Calling good()...".as_ptr());
    good(good_data);
    printLine(c"Finished good()".as_ptr());
    printLine(c"Calling bad()...".as_ptr());
    bad(bad_data);
    printLine(c"Finished bad()".as_ptr());
}
