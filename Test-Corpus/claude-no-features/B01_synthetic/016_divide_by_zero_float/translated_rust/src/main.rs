// Translation of c_src/src/main.c to Rust.
// Reproduces C behavior, including stdin reading semantics of fgets and
// the platform-typical outcome of casting non-finite/out-of-range doubles
// to int (matching x86_64 Linux gcc's cvttsd2si: returns INT_MIN).

use std::io::{self, Read, Write};

const CHAR_ARRAY_SIZE: usize = 20;

fn print_line(line: &str) {
    // Matches printf("%s\n", line);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(line.as_bytes());
    let _ = handle.write_all(b"\n");
}

fn print_int_line(int_number: i32) {
    // Matches printf("%d\n", intNumber);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", int_number);
}

/// Mimic C's fgets(buffer, CHAR_ARRAY_SIZE, stdin):
/// Reads up to CHAR_ARRAY_SIZE-1 bytes (or until newline, inclusive, or EOF).
/// Returns None if no bytes were read before EOF, otherwise Some(bytes)
/// (newline is included if present).
fn fgets_stdin(max_chars: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::with_capacity(max_chars);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    // We can read at most max_chars - 1 bytes (the rest is for the null terminator).
    while buf.len() < max_chars - 1 {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
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
        Some(buf)
    }
}

/// Mimic C's atof: skip leading whitespace, parse optional sign, digits,
/// optional '.' and digits, optional exponent. Return 0.0 if no conversion.
fn c_atof(bytes: &[u8]) -> f64 {
    let mut i = 0usize;
    let n = bytes.len();
    // Skip whitespace (matches isspace in C locale: space, tab, LF, VT, FF, CR)
    while i < n {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\n' || b == 0x0B || b == 0x0C || b == b'\r' {
            i += 1;
        } else {
            break;
        }
    }
    let start = i;
    // Optional sign
    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    // Integer digits
    let int_digits_start = i;
    while i < n && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let has_int_digits = i > int_digits_start;
    // Fractional part
    let mut has_frac_digits = false;
    if i < n && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < n && bytes[i].is_ascii_digit() {
            i += 1;
        }
        has_frac_digits = i > frac_start;
    }
    if !has_int_digits && !has_frac_digits {
        return 0.0;
    }
    // Optional exponent
    if i < n && (bytes[i] == b'e' || bytes[i] == b'E') {
        let exp_save = i;
        i += 1;
        if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_digits_start = i;
        while i < n && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_digits_start {
            // No exponent digits — discard the 'e' and trailing sign.
            i = exp_save;
        }
    }
    // Parse the slice [start..i] as f64.
    match std::str::from_utf8(&bytes[start..i]) {
        Ok(s) => s.parse::<f64>().unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

/// Cast a f64 to i32 with the same observable behavior as gcc on x86_64
/// Linux: NaN, +/-Inf, and out-of-range values produce INT_MIN
/// (0x80000000 / -2147483648), matching the cvttsd2si "invalid" result.
fn double_to_int_c(f: f64) -> i32 {
    if f.is_nan() || !f.is_finite() {
        return i32::MIN;
    }
    if f >= 2147483648.0_f64 {
        return i32::MIN;
    }
    if f < -2147483648.0_f64 {
        return i32::MIN;
    }
    f as i32
}

fn bad() {
    let mut data: f32 = 0.0_f32;
    {
        match fgets_stdin(CHAR_ARRAY_SIZE) {
            Some(buf) => {
                data = c_atof(&buf) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        // (int)(100.0 / data) — 100.0 is a C double; data is promoted to double.
        let result = double_to_int_c(100.0_f64 / data as f64);
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0_f32;
    let result = double_to_int_c(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g() {
    let mut data: f32 = 0.0_f32;
    {
        match fgets_stdin(CHAR_ARRAY_SIZE) {
            Some(buf) => {
                data = c_atof(&buf) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    if (data as f64).abs() > 0.000001_f64 {
        let result = double_to_int_c(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
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
