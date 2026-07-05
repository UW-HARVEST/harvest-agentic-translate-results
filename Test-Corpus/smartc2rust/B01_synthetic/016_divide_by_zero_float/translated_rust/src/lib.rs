
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};
use std::io::BufRead;

fn rust_print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn rust_print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn read_line_from_stdin() -> Option<String> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(_) => None,
    }
}

/// Mimics C's `atof`: parses a leading numeric prefix from the input.
/// Returns 0.0 when no valid numeric prefix is present.
fn parse_atof(s: &str) -> f64 {
    let trimmed = s.trim_start();
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;

    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }

    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }

    if saw_digit && i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let mut exp_digit = false;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
            exp_digit = true;
        }
        if exp_digit {
            i = j;
        }
    }

    if !saw_digit {
        return 0.0;
    }

    trimmed[..i].parse::<f64>().unwrap_or(0.0)
}

fn read_float_from_stdin_or_default(default: f32) -> f32 {
    match read_line_from_stdin() {
        Some(s) => parse_atof(&s) as f32,
        None => {
            rust_print_line(Some("fgets() failed."));
            default
        }
    }
}

fn rust_bad() {
    let data: f32 = read_float_from_stdin_or_default(0.0);
    let result = (100.0_f64 / data as f64) as i32;
    rust_print_int_line(result);
}

fn rust_good_b2g() {
    let data: f32 = read_float_from_stdin_or_default(0.0);
    if (data as f64).abs() > 0.000001 {
        let result = (100.0_f64 / data as f64) as i32;
        rust_print_int_line(result);
    } else {
        rust_print_line(Some("This would result in a divide by zero"));
    }
}

fn rust_good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0_f64 / data as f64) as i32;
    rust_print_int_line(result);
}

fn rust_good() {
    rust_good_g2b();
    rust_good_b2g();
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    rust_print_line(Some("Calling good()..."));
    rust_good();
    rust_print_line(Some("Finished good()"));
    rust_print_line(Some("Calling bad()..."));
    rust_bad();
    rust_print_line(Some("Finished bad()"));
    0
}