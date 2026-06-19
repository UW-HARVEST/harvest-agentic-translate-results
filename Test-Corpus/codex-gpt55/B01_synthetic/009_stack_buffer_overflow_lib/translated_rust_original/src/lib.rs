use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const STRING_LINE_FORMAT: &[u8] = b"%s\n\0";
const INT_LINE_FORMAT: &[u8] = b"%d\n\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";
const NEGATIVE_INDEX_ERROR: &[u8] = b"ERROR: Array index is negative.\0";
const OUT_OF_BOUNDS_ERROR: &[u8] = b"ERROR: Array index is out-of-bounds\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(STRING_LINE_FORMAT.as_ptr().cast::<c_char>(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(INT_LINE_FORMAT.as_ptr().cast::<c_char>(), int_number);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_int) {
    let mut buffer = [0 as c_int; 10];

    if data >= 0 {
        unsafe {
            buffer.as_mut_ptr().wrapping_offset(data as isize).write(1);
        }

        for value in buffer {
            unsafe {
                printIntLine(value);
            }
        }
    } else {
        unsafe {
            printLine(NEGATIVE_INDEX_ERROR.as_ptr().cast::<c_char>());
        }
    }
}

fn good_g2b() {
    let data = 7;
    let mut buffer = [0 as c_int; 10];

    if data >= 0 {
        buffer[data as usize] = 1;

        for value in buffer {
            unsafe {
                printIntLine(value);
            }
        }
    } else {
        unsafe {
            printLine(NEGATIVE_INDEX_ERROR.as_ptr().cast::<c_char>());
        }
    }
}

fn good_b2g(data: c_int) {
    let mut buffer = [0 as c_int; 10];

    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;

        for value in buffer {
            unsafe {
                printIntLine(value);
            }
        }
    } else {
        unsafe {
            printLine(OUT_OF_BOUNDS_ERROR.as_ptr().cast::<c_char>());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    unsafe {
        printLine(CALLING_GOOD.as_ptr().cast::<c_char>());
        good(good_data);
        printLine(FINISHED_GOOD.as_ptr().cast::<c_char>());
        printLine(CALLING_BAD.as_ptr().cast::<c_char>());
        bad(bad_data);
        printLine(FINISHED_BAD.as_ptr().cast::<c_char>());
    }
}
