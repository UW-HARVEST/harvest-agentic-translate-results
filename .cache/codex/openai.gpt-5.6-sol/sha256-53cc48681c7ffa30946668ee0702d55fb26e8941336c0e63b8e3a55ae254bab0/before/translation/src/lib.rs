use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const LINE_FORMAT: &[u8] = b"%s\n\0";
const HEX_CHAR_FORMAT: &[u8] = b"%02x\n\0";
const TOO_LARGE_MESSAGE: &[u8] = b"data value is too large to perform arithmetic safely.\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(LINE_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    unsafe {
        printf(HEX_CHAR_FORMAT.as_ptr().cast(), c_int::from(char_hex));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data = c_char::MAX;
    if data > 0 {
        let result = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn good_g2b() {
    let data: c_char = 2;
    if data > 0 {
        let result = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn good_b2g() {
    let data = c_char::MAX;
    if data > 0 {
        if data < (c_char::MAX / 2) {
            let result = data.wrapping_mul(2);
            printHexCharLine(result);
        } else {
            unsafe {
                printLine(TOO_LARGE_MESSAGE.as_ptr().cast());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    good_g2b();
    good_b2g();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
