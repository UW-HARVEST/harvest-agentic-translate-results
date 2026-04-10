use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    // C promotes char to int (sign-extends), then %02x prints as unsigned hex
    let promoted: c_int = char_hex as c_int;
    unsafe {
        printf(b"%02x\n\0".as_ptr() as *const c_char, promoted);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data: c_char = c_char::MAX;
    if data > 0 {
        let result: c_char = (data as i8).wrapping_mul(2) as c_char;
        printHexCharLine(result);
    }
}

fn good_g2b() {
    let data: c_char = 2;
    if data > 0 {
        let result: c_char = (data as i8).wrapping_mul(2) as c_char;
        printHexCharLine(result);
    }
}

fn good_b2g() {
    let data: c_char = c_char::MAX;
    if data > 0 {
        if data < (c_char::MAX / 2) {
            let result: c_char = (data as i8).wrapping_mul(2) as c_char;
            printHexCharLine(result);
        } else {
            printLine(b"data value is too large to perform arithmetic safely.\0".as_ptr() as *const c_char);
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
