use std::io::{self, BufRead};
use std::os::raw::c_char;
use std::ffi::CStr;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) }.to_str().unwrap_or("");
        print_line(s);
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(n: i32) {
    print_int_line(n);
}

/// C-like atoi: skip leading whitespace, parse optional sign + digits.
fn c_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let mut neg = false;
    match chars.peek() {
        Some('+') => { chars.next(); }
        Some('-') => { neg = true; chars.next(); }
        _ => {}
    }
    let mut result: i32 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            result = result.wrapping_mul(10).wrapping_add(d as i32);
        } else {
            break;
        }
    }
    if neg { result.wrapping_neg() } else { result }
}

fn read_input() -> Result<i32, ()> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => Err(()),
        Ok(_) => {
            if line.len() > 13 {
                line.truncate(13);
            }
            Ok(c_atoi(&line))
        }
        Err(_) => Err(()),
    }
}

#[no_mangle]
pub extern "C" fn bad() {
    let mut data: i32 = -1;
    match read_input() {
        Ok(v) => data = v,
        Err(()) => print_line("fgets() failed."),
    }
    {
        let mut buffer = [0i32; 10];
        if data >= 0 {
            unsafe {
                *buffer.as_mut_ptr().offset(data as isize) = 1;
            }
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

fn good_g2b() {
    #[allow(unused_assignments)]
    let mut data: i32 = -1;
    data = 7;
    {
        let mut buffer = [0i32; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

fn good_b2g() {
    let mut data: i32 = -1;
    match read_input() {
        Ok(v) => data = v,
        Err(()) => print_line("fgets() failed."),
    }
    {
        let mut buffer = [0i32; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is out-of-bounds");
        }
    }
}

#[no_mangle]
pub extern "C" fn good() {
    good_g2b();
    good_b2g();
}

pub fn run_main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) -> i32 {
    run_main();
    0
}
