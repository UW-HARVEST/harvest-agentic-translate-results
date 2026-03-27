use std::ffi::{c_char, c_int, CStr};

const CHAR_MAX: i8 = i8::MAX; // 127

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap_or(""));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    // C promotes char to int, then %02x prints as unsigned int
    let as_int = char_hex as i32;
    let as_uint = as_int as u32;
    println!("{:02x}", as_uint);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn good_b2g() {
    let data: i8;
    // data = ' ' then reassigned — matches C source
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
            printHexCharLine(result);
        } else {
            printLine(c"data value is too large to perform arithmetic safely.".as_ptr());
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
