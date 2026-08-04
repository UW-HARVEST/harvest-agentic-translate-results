#![no_main]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

macro_rules! c_printf {
    ($fmt:expr) => {
        unsafe { printf(CString::new($fmt).unwrap().as_ptr()); }
    };
    ($fmt:expr, $($arg:expr),+) => {
        unsafe { printf(CString::new($fmt).unwrap().as_ptr(), $($arg),+); }
    };
}

fn strtol(s: &str) -> (i64, usize) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let neg = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
        false
    } else {
        false
    };
    let digit_start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == digit_start {
        return (0, 0);
    }
    if neg {
        val = val.wrapping_neg();
    }
    (val, i)
}

#[no_mangle]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc > 4 || argc == 1 {
        c_printf!("Error: there should be one to three arguments passed:\n");
        c_printf!("<string> [start] [stop]\n");
        return 1;
    }

    let args: Vec<&str> = (0..argc as usize)
        .map(|i| unsafe { CStr::from_ptr(*argv.add(i)).to_str().unwrap() })
        .collect();

    let s = args[1];
    let len = s.len();

    let start: i32;
    let stop: i32;
    let mut prev_end_equals_input = false;

    if argc >= 3 {
        let (val, consumed) = strtol(args[2]);
        start = val as i32;
        if consumed == 0 {
            prev_end_equals_input = true;
        }
        if prev_end_equals_input {
            c_printf!("Second argument must be an integer!");
            return 1;
        }
        if (start as u64) > (len as u64) {
            c_printf!("Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let (val, _consumed) = strtol(args[3]);
        stop = val as i32;
        if prev_end_equals_input {
            c_printf!("Third argument must be an integer!");
            return 1;
        }
        if (stop as u64) > (len as u64) {
            c_printf!("Error: stop is off the end of the string!\n");
            return 1;
        }
        if stop <= start {
            c_printf!("Error: stop must come after start!\n");
            return 1;
        }
    } else {
        stop = len as i32;
    }

    let width = (stop - start) as usize;
    let begin = start as usize;
    // C: printf("%.*s\n", stop - start, argv[1] + start)
    let substr = &s[begin..begin + width];
    let c_substr = CString::new(substr).unwrap();
    c_printf!("%.*s\n", width as c_int, c_substr.as_ptr());
    0
}
