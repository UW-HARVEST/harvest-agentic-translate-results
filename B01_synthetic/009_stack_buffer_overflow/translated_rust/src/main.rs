use std::io::{self, Read};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Mimics C's atoi: skip leading whitespace, parse optional sign + digits.
fn c_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('-') => { chars.next(); true }
        Some('+') => { chars.next(); false }
        _ => false,
    };
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

/// Mimics fgets(buf, size, stdin): reads up to size-1 bytes, stops at newline (included).
/// Returns None on EOF with no data read (like fgets returning NULL).
fn fgets(size: usize) -> Option<String> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf = Vec::new();
    let limit = size - 1;
    let mut byte = [0u8; 1];
    for _ in 0..limit {
        match handle.read(&mut byte) {
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
        None
    } else {
        Some(String::from_utf8_lossy(&buf).into_owned())
    }
}

fn bad() {
    let mut data: i32 = -1;
    if let Some(input) = fgets(14) {
        data = c_atoi(&input);
    } else {
        print_line("fgets() failed.");
    }
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

fn good_g2b() {
    let data: i32 = 7;
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

fn good_b2g() {
    let mut data: i32 = -1;
    if let Some(input) = fgets(14) {
        data = c_atoi(&input);
    } else {
        print_line("fgets() failed.");
    }
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

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
