#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const STRING_FORMAT: &[u8] = b"%s\n\0";
const INT_FORMAT: &[u8] = b"%d\n\0";
const NEGATIVE_INDEX: &[u8] = b"ERROR: Array index is negative.\0";
const OUT_OF_BOUNDS: &[u8] = b"ERROR: Array index is out-of-bounds\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(STRING_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(INT_FORMAT.as_ptr().cast(), intNumber);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];

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
            printLine(NEGATIVE_INDEX.as_ptr().cast());
        }
    }
}

fn goodG2B() {
    let data: c_int = 7;
    let mut buffer: [c_int; 10] = [0; 10];

    if data >= 0 {
        buffer[data as usize] = 1;
        for value in buffer {
            unsafe {
                printIntLine(value);
            }
        }
    } else {
        unsafe {
            printLine(NEGATIVE_INDEX.as_ptr().cast());
        }
    }
}

fn goodB2G(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];

    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for value in buffer {
            unsafe {
                printIntLine(value);
            }
        }
    } else {
        unsafe {
            printLine(OUT_OF_BOUNDS.as_ptr().cast());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_int) {
    goodG2B();
    goodB2G(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(goodData: c_int, badData: c_int) {
    unsafe {
        printLine(CALLING_GOOD.as_ptr().cast());
        good(goodData);
        printLine(FINISHED_GOOD.as_ptr().cast());
        printLine(CALLING_BAD.as_ptr().cast());
        bad(badData);
        printLine(FINISHED_BAD.as_ptr().cast());
    }
}
