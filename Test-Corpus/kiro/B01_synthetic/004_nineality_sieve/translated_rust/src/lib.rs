use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn strtol_parse(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let neg = bytes[i] == b'-';
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        val = val.wrapping_neg();
    }
    Some(val as i32)
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return 1;
    }

    let arg1 = unsafe { CStr::from_ptr(*argv.offset(1)) };
    let arg1_str = match arg1.to_str() {
        Ok(s) => s,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            return 1;
        }
    };

    let val = match strtol_parse(arg1_str) {
        Some(v) => v,
        None => {
            println!("Error: first argument must be an integer!");
            return 1;
        }
    };

    let mut val = val;
    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }

    0
}
