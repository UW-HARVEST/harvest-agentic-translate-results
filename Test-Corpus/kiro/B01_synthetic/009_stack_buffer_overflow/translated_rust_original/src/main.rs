use std::io::{self, BufRead};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Mimics C's fgets(buf, 14, stdin): reads up to 13 bytes (including newline),
/// returns None on EOF/error.
fn fgets_14(reader: &mut impl BufRead) -> Option<String> {
    let mut buf = Vec::new();
    for _ in 0..13 {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break,   // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() && true {
        // fgets returns NULL only when no bytes read AND at EOF
        // But to match: if we got 0 bytes, treat as failure
        // Actually fgets returns NULL on EOF with no chars read
        return if buf.is_empty() { None } else { Some(String::new()) };
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Mimics C's atoi: skip whitespace, optional sign, parse digits, return 0 on no digits.
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

fn bad(reader: &mut impl BufRead) {
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
            // Intentional buffer overflow bug — reproduce C behavior exactly
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

fn good(reader: &mut impl BufRead) {
    good_g2b();
    good_b2g(reader);
}

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    print_line("Calling good()...");
    good(&mut reader);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut reader);
    print_line("Finished bad()");
}
