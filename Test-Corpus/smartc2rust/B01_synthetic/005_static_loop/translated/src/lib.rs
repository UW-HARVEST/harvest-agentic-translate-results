
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

use std::sync::Mutex;
use std::ffi::CStr;

static STATIC_SUM_STATE: Mutex<i32> = Mutex::new(0);

fn static_sum(update: i32) -> i32 {
    let mut sum = STATIC_SUM_STATE.lock().unwrap();
    *sum += update;
    *sum
}

/// Emulate C's `strtol` base-10 behavior: parse a leading optional sign and
/// consecutive ASCII digits. Returns the parsed value and the number of
/// bytes consumed (0 if nothing was parsed, matching C's `end == nptr`).
fn parse_strtol_base10(s: &str) -> (i32, usize) {
    let bytes = s.as_bytes();
    let mut idx = 0;

    // Skip leading whitespace (strtol behavior)
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    let after_ws = idx;

    // Optional sign
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }
    let digits_start = idx;

    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }

    if digits_start == idx {
        // Nothing parsed -> "end == argv[1]" in C
        return (0, 0);
    }

    let value: i32 = s[after_ws..idx].parse().unwrap_or(0);
    (value, idx)
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    // Safely materialize argv into owned Rust strings at the FFI boundary.
    let args: Vec<String> = (0..argc as isize)
        .map(|i| {
            let cstr_ptr = unsafe { *argv.offset(i) };
            if cstr_ptr.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(cstr_ptr) }
                    .to_string_lossy()
                    .into_owned()
            }
        })
        .collect();

    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return 1;
    }

    let (stride, consumed) = parse_strtol_base10(&args[1]);
    if consumed == 0 {
        println!("Error: first argument must be an integer!");
        return 1;
    }

    for i in 0..10 {
        println!("{}", static_sum(i * stride));
    }

    0
}