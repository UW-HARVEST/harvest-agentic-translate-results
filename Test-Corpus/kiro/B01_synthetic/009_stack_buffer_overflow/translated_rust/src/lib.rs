use std::ffi::CStr;
use std::io::{self, BufRead};
use std::os::raw::{c_char, c_int};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

fn fgets_14(reader: &mut impl BufRead) -> Option<String> {
    let mut buf = Vec::new();
    for _ in 0..13 {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn c_atoi(s: &str) -> i32 {
    let mut chars = s.chars().peekable();
    while chars.peek().map_or(false, |c| c.is_ascii_whitespace()) {
        chars.next();
    }
    let neg = match chars.peek() {
        Some('-') => { chars.next(); true }
        Some('+') => { chars.next(); false }
        _ => false,
    };
    let mut result: i32 = 0;
    while let Some(&c) = chars.peek() {
        if let Some(d) = c.to_digit(10) {
            result = result.wrapping_mul(10).wrapping_add(d as i32);
            chars.next();
        } else {
            break;
        }
    }
    if neg { result.wrapping_neg() } else { result }
}

fn bad_impl(reader: &mut impl BufRead) {
    let mut data: i32 = -1;
    {
        if let Some(input) = fgets_14(reader) {
            data = c_atoi(&input);
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let mut buffer = [0i32; 10];
        if data >= 0 {
            unsafe {
                let ptr = buffer.as_mut_ptr();
                *ptr.offset(data as isize) = 1;
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

fn good_b2g(reader: &mut impl BufRead) {
    let mut data: i32 = -1;
    {
        if let Some(input) = fgets_14(reader) {
            data = c_atoi(&input);
        } else {
            print_line("fgets() failed.");
        }
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

fn good_impl(reader: &mut impl BufRead) {
    good_g2b();
    good_b2g(reader);
}

// --- #[no_mangle] FFI exports ---

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) }.to_string_lossy();
        println!("{}", s);
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(n: c_int) {
    println!("{}", n);
}

#[no_mangle]
pub extern "C" fn bad() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    bad_impl(&mut reader);
}

#[no_mangle]
pub extern "C" fn good() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    good_impl(&mut reader);
}

/// Exported as `main` to match the C .so symbol table.
/// Note: named `driver_main` internally to avoid conflict with Rust's main,
/// but exported as "main" via the link_name attribute.
#[export_name = "main"]
pub extern "C" fn driver_main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    print_line("Calling good()...");
    good_impl(&mut reader);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad_impl(&mut reader);
    print_line("Finished bad()");
    0
}
