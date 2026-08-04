// Translation of c_src/src/driver.c to Rust.
//
// The original C code is a shared library exposing a `driver(int useGood)`
// function with no `main`. To produce an executable, this Rust program reads
// a single integer from stdin (matching scanf("%d") whitespace-skipping
// behavior, which reads across newlines) and passes it to `driver`.

use std::io::{self, Read, Write};

#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    // Original C uses alloca(10) which allocates only 10 bytes (a bug),
    // then writes 10 ints into it. We reproduce the observable output:
    // the loop fills indices 0..10 with zeros and prints data[0] which is 0.
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

fn driver(use_good: i32) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

/// Read a single integer from stdin in a manner equivalent to C's
/// `scanf("%d", &x)`: skip leading whitespace (including newlines), then
/// parse an optional sign followed by decimal digits. Returns the parsed
/// value, or 0 if no integer could be read (matching the behavior of an
/// uninitialized variable when the original program ignores scanf's return).
fn read_int_scanf() -> i32 {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return 0;
    }

    let mut i = 0usize;
    // Skip leading whitespace (matches C's isspace handling for %d).
    while i < buf.len() && (buf[i] as char).is_ascii_whitespace() {
        i += 1;
    }

    if i >= buf.len() {
        return 0;
    }

    let mut sign: i64 = 1;
    if buf[i] == b'+' {
        i += 1;
    } else if buf[i] == b'-' {
        sign = -1;
        i += 1;
    }

    let start = i;
    while i < buf.len() && buf[i].is_ascii_digit() {
        i += 1;
    }

    if start == i {
        return 0;
    }

    let mut value: i64 = 0;
    for &b in &buf[start..i] {
        value = value.wrapping_mul(10).wrapping_add((b - b'0') as i64);
    }
    value = value.wrapping_mul(sign);
    value as i32
}

fn main() {
    let use_good = read_int_scanf();
    driver(use_good);
    // Ensure stdout is flushed before exit (matches stdio's behavior on
    // normal program termination).
    let _ = io::stdout().flush();
}
