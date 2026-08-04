use std::io::{self, Read, Write};

fn print_line(line: &str) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", line);
}

fn print_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", int_number);
}

/// Mimic C's fgets(buf, n, stdin):
/// - reads at most n-1 bytes from stdin
/// - stops after a newline (which is included in the result)
/// - returns None if no characters are read before EOF
fn fgets(n: usize) -> Option<Vec<u8>> {
    if n == 0 {
        return None;
    }
    let max_chars = n - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(max_chars);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    let mut read_any = false;
    while buf.len() < max_chars {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                read_any = true;
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if !read_any {
        return None;
    }
    Some(buf)
}

/// Mimic C's atoi: skip leading whitespace, optional sign, parse digits,
/// stop at first non-digit. Returns 0 if no digits are found.
fn atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace in C locale)
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        // Use wrapping arithmetic to mirror C's behavior on overflow.
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }
    result.wrapping_mul(sign)
}

fn bad() {
    let mut data: i32 = -1;
    // char inputBuffer[14] = "";  fgets(inputBuffer, 14, stdin)
    match fgets(14) {
        Some(input) => {
            data = atoi(&input);
        }
        None => {
            print_line("fgets() failed.");
        }
    }
    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

/* goodG2B uses the GoodSource with the BadSink */
fn good_g2b() {
    let mut data: i32 = -1;
    data = 7;
    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

/* goodB2G uses the BadSource with the GoodSink */
fn good_b2g() {
    let mut data: i32 = -1;
    match fgets(14) {
        Some(input) => {
            data = atoi(&input);
        }
        None => {
            print_line("fgets() failed.");
        }
    }
    let mut buffer: [i32; 10] = [0; 10];
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
