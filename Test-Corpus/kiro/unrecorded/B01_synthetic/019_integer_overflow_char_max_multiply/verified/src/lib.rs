use std::ffi::CStr;
use std::os::raw::c_char;

const CHAR_MAX: i8 = i8::MAX;

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap_or(""));
    }
}

#[no_mangle]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    let val = char_hex as i32 as u32;
    println!("{:02x}", val);
}

#[no_mangle]
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
    let data: i8 = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
            printHexCharLine(result);
        } else {
            let msg = std::ffi::CString::new("data value is too large to perform arithmetic safely.").unwrap();
            printLine(msg.as_ptr());
        }
    }
}

#[no_mangle]
pub extern "C" fn good() {
    good_g2b();
    good_b2g();
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);
    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
