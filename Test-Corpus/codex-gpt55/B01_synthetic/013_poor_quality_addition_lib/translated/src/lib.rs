#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const STR_LINE_FMT: &[u8] = b"%s\n\0";
const INT_LINE_FMT: &[u8] = b"%d\n\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(STR_LINE_FMT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(INT_LINE_FMT.as_ptr().cast(), intNumber);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let intOne: c_int = 1;
    let intTwo: c_int = 1;
    let intSum: c_int = 0;

    printIntLine(intSum);
    let _ = intOne + intTwo;
    printIntLine(intSum);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let intOne: c_int = 1;
    let intTwo: c_int = 1;
    let mut intSum: c_int = 0;

    printIntLine(intSum);
    intSum = intOne + intTwo;
    printIntLine(intSum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    printLine(CALLING_GOOD.as_ptr().cast());
    good();
    printLine(FINISHED_GOOD.as_ptr().cast());
    printLine(CALLING_BAD.as_ptr().cast());
    bad();
    printLine(FINISHED_BAD.as_ptr().cast());
}
