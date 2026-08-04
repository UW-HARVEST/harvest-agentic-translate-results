use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn strtol_partial(s: &str) -> (i32, bool) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let negative = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else {
        if i < bytes.len() && bytes[i] == b'+' {
            i += 1;
        }
        false
    };
    let digit_start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val * 10 + (bytes[i] - b'0') as i64;
        i += 1;
    }
    let parsed_any = i > digit_start;
    if negative {
        val = -val;
    }
    (val as i32, parsed_any)
}

/// Exported as C-compatible `main` only when building the cdylib (not during `cargo test --lib`).
#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return 1;
    }

    let arg1 = unsafe { CStr::from_ptr(*argv.offset(1)) };
    let s = arg1.to_str().unwrap_or("");

    let (mut val, parsed) = strtol_partial(s);
    if !parsed {
        println!("Error: first argument must be an integer!");
        return 1;
    }

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }

    0
}
