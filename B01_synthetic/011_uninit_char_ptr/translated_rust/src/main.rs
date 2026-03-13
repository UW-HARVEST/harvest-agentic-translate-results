use std::ffi::CStr;
use std::io::{self, BufRead};
use std::mem::MaybeUninit;
use std::os::raw::c_char;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            let s = CStr::from_ptr(line);
            println!("{}", s.to_str().unwrap_or(""));
        }
    }
}

fn bad() {
    unsafe {
        let data: *const c_char = MaybeUninit::uninit().assume_init();
        print_line(data);
    }
}

fn good() {
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    print_line(data);
}

fn main() {
    let mut x: i32 = 0;
    let stdin = io::stdin();
    // Match scanf("%d", &x): read tokens skipping whitespace
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        for token in line.split_whitespace() {
            if let Ok(val) = token.parse::<i32>() {
                x = val;
                // scanf stops after first successful conversion
                if x != 0 {
                    good();
                } else {
                    bad();
                }
                return;
            }
        }
    }
    // If no integer was read, x stays 0 (scanf failure), call bad()
    if x != 0 {
        good();
    } else {
        bad();
    }
}
