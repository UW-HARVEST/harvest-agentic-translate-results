#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const PRINT_LINE_FMT: &[u8] = b"%s\n\0";
const PRINT_INT_FMT: &[u8] = b"%d\n\0";
const NEGATIVE_INDEX_ERROR: &[u8] = b"ERROR: Array index is negative.\0";
const OOB_INDEX_ERROR: &[u8] = b"ERROR: Array index is out-of-bounds\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(PRINT_LINE_FMT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(PRINT_INT_FMT.as_ptr().cast(), intNumber);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_int) {
    let mut i: c_int;
    let mut buffer = [0 as c_int; 10];

    if data >= 0 {
        unsafe {
            buffer.as_mut_ptr().add(data as usize).write(1);
        }

        i = 0;
        while i < 10 {
            unsafe {
                printIntLine(buffer[i as usize]);
            }
            i += 1;
        }
    } else {
        unsafe {
            printLine(NEGATIVE_INDEX_ERROR.as_ptr().cast());
        }
    }
}

unsafe fn goodG2B() {
    let data: c_int = 7;
    let mut i: c_int;
    let mut buffer = [0 as c_int; 10];

    if data >= 0 {
        buffer[data as usize] = 1;

        i = 0;
        while i < 10 {
            unsafe {
                printIntLine(buffer[i as usize]);
            }
            i += 1;
        }
    } else {
        unsafe {
            printLine(NEGATIVE_INDEX_ERROR.as_ptr().cast());
        }
    }
}

unsafe fn goodB2G(data: c_int) {
    let mut i: c_int;
    let mut buffer = [0 as c_int; 10];

    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;

        i = 0;
        while i < 10 {
            unsafe {
                printIntLine(buffer[i as usize]);
            }
            i += 1;
        }
    } else {
        unsafe {
            printLine(OOB_INDEX_ERROR.as_ptr().cast());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_int) {
    unsafe {
        goodG2B();
        goodB2G(data);
    }
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
