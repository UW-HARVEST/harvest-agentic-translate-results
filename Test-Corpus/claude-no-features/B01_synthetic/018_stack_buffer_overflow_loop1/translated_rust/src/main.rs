use std::io::{self, Read};

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn bad() {
    // The C version uses alloca(10) — only 10 bytes — and treats it as 10 ints,
    // which is undefined behavior but in practice writes 0s and reads back 0.
    // We reproduce the observable output (printing data[0] == 0) using safe Rust.
    let mut data = [0i32; 10];
    let source = [0i32; 10];
    for i in 0..10usize {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let mut data = [0i32; 10];
    let source = [0i32; 10];
    for i in 0..10usize {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn main() {
    // Match scanf("%d", &x): if no integer is read, x remains 0.
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let mut x: i32 = 0;
    // Mimic scanf %d: skip leading whitespace, read optional sign, then digits.
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i > start {
        let s = std::str::from_utf8(&bytes[start..i]).unwrap_or("0");
        if let Ok(v) = s.parse::<i64>() {
            let v = sign * v;
            // Truncate to i32 like C's %d.
            x = v as i32;
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
