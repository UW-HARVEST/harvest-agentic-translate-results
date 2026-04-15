use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

fn print_hex_char_line(char_hex: c_char) {
    println!("{:02x}", char_hex as c_int);
}

fn bad() {
    let data: c_char = c_char::MAX;
    if data > 0 {
        let result = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: c_char = 2;
    if data > 0 {
        let result = data * 2;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: c_char = b' ' as c_char;
    data = c_char::MAX;
    if data > 0 {
        if data < (c_char::MAX / 2) {
            let result = data * 2;
            print_hex_char_line(result);
        } else {
            let msg = b"data value is too large to perform arithmetic safely.\0";
            print_line(msg.as_ptr() as *const c_char);
        }
    }
}

fn good() {
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
