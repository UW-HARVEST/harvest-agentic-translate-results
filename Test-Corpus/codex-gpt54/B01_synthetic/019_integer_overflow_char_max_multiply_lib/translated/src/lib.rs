#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const PRINT_LINE_FORMAT: &[u8] = b"%s\n\0";
const PRINT_HEX_FORMAT: &[u8] = b"%02x\n\0";
const GOOD_B2G_MESSAGE: &[u8] = b"data value is too large to perform arithmetic safely.\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(PRINT_LINE_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_char) {
    unsafe {
        printf(PRINT_HEX_FORMAT.as_ptr().cast(), charHex as c_int);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data = c_char::MAX;
    if (data as c_int) > 0 {
        let result = data.wrapping_mul(2 as c_char);
        printHexCharLine(result);
    }
}

fn goodG2B() {
    let data = 2 as c_char;
    if (data as c_int) > 0 {
        let result = data.wrapping_mul(2 as c_char);
        printHexCharLine(result);
    }
}

#[allow(unused_assignments)]
fn goodB2G() {
    let mut data = b' ' as c_char;
    data = c_char::MAX;
    if (data as c_int) > 0 {
        if (data as c_int) < ((c_char::MAX as c_int) / 2) {
            let result = data.wrapping_mul(2 as c_char);
            printHexCharLine(result);
        } else {
            printLine(GOOD_B2G_MESSAGE.as_ptr().cast());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    goodG2B();
    goodB2G();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
