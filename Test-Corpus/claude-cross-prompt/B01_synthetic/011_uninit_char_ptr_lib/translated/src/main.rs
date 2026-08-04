// Translation of c_src/src/driver.c to Rust.
//
// The original C code is a shared library exposing a single `driver(int useGood)`
// function. There is no `main` in the C source, so the original library has no
// inherent stdin/stdout behavior. To make this an executable, we read a single
// integer from stdin (mirroring `scanf("%d", &useGood)`) and pass it to driver().
// If no integer can be read, no further action is taken (matching the case where
// scanf fails to populate its argument and the caller does nothing meaningful).

use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    // Mirror C: if (line != NULL) printf("%s\n", line);
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn bad() {
    // Mirror C: char *data; printLine(data);
    // `data` is an uninitialized pointer in the C source. In safe Rust we can't
    // express genuine "uninitialized" memory, so we model the most common
    // observable behavior under typical compilers/runtimes: the pointer starts
    // as NULL, which causes printLine to do nothing.
    let data: Option<&str> = None;
    print_line(data);
}

fn good() {
    // Mirror C: char *data; data = "string"; printLine(data);
    let data: Option<&str> = Some("string");
    print_line(data);
}

fn driver(use_good: i32) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

fn read_int_from_stdin() -> Option<i32> {
    // Mirror C's scanf("%d", ...) for an int: skip leading whitespace
    // (including newlines), then read an optional sign followed by decimal digits.
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return None;
    }

    let bytes = buf.as_bytes();
    let mut i = 0usize;

    // Skip whitespace (matches isspace: space, \t, \n, \v, \f, \r)
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\n' || b == 0x0B || b == 0x0C || b == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    if i >= bytes.len() {
        return None;
    }

    let mut negative = false;
    if bytes[i] == b'+' {
        i += 1;
    } else if bytes[i] == b'-' {
        negative = true;
        i += 1;
    }

    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    if start == i {
        return None;
    }

    let digit_str = std::str::from_utf8(&bytes[start..i]).ok()?;
    let value: i64 = digit_str.parse().ok()?;
    let signed = if negative { -value } else { value };
    Some(signed as i32)
}

fn main() {
    if let Some(use_good) = read_int_from_stdin() {
        driver(use_good);
    }
}
