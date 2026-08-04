use std::io::{self, Read};

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    // In C, data = alloca(10) allocates only 10 bytes, then is used as int*
    // and writes 10 ints (40 bytes). This is a buffer overflow (CWE-806/121),
    // but the program still prints data[0] which equals source[0] = 0.
    let mut data: [i32; 10] = [0; 10];
    let source: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let mut data: [i32; 10] = [0; 10];
    let source: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn read_int_scanf() -> i32 {
    // Mimic scanf("%d", &x): skip leading whitespace, read optional sign, then digits.
    // If parse fails, x remains 0 (matching the initialized value in C).
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return 0;
    }
    let mut idx = 0;
    // skip whitespace
    while idx < buf.len() && (buf[idx] as char).is_ascii_whitespace() {
        idx += 1;
    }
    let start = idx;
    if idx < buf.len() && (buf[idx] == b'+' || buf[idx] == b'-') {
        idx += 1;
    }
    let digit_start = idx;
    while idx < buf.len() && (buf[idx] as char).is_ascii_digit() {
        idx += 1;
    }
    if idx == digit_start {
        // no digits found, scanf would fail; x stays 0
        return 0;
    }
    let s = std::str::from_utf8(&buf[start..idx]).unwrap_or("0");
    // Match C scanf %d: parse as int, on overflow behavior is undefined in C;
    // we'll saturate or wrap. Use wrapping via i64 then cast.
    match s.parse::<i64>() {
        Ok(v) => v as i32,
        Err(_) => 0,
    }
}

fn main() {
    let x = read_int_scanf();
    if x != 0 {
        good();
    } else {
        bad();
    }
}
