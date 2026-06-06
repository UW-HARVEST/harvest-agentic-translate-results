// Translated from c_src/src/main.c
// Reproduces the same output, byte-for-byte.

use std::io::{self, Read, Write};

const CHAR_ARRAY_SIZE: usize = 20;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Mirrors C's `fgets(buf, n, stdin)` semantics:
/// reads up to n-1 bytes, stopping after a newline (which is kept) or EOF.
/// Returns `None` if EOF is reached and no bytes were read (matches `fgets`
/// returning NULL on immediate EOF).
fn c_fgets(stdin: &mut impl Read, max_size: usize) -> Option<Vec<u8>> {
    if max_size == 0 {
        return None;
    }
    let mut buf: Vec<u8> = Vec::with_capacity(max_size);
    let mut byte = [0u8; 1];
    while buf.len() < max_size - 1 {
        match stdin.read(&mut byte) {
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

/// Mirrors C's `atof`: parses optional leading whitespace, optional sign,
/// digits with optional decimal point, optional exponent. Stops at the first
/// character that is not part of the recognised number form. Returns 0.0 if
/// no conversion can be performed.
fn c_atof(bytes: &[u8]) -> f64 {
    let mut i = 0;
    let n = bytes.len();

    // Skip leading whitespace (C's isspace: space, \t, \n, \v, \f, \r)
    while i < n && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }
    let start = i;

    // Optional sign
    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // Integer part
    let int_start = i;
    while i < n && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let had_int_digits = i > int_start;

    // Fractional part
    let mut had_frac_digits = false;
    if i < n && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < n && bytes[i].is_ascii_digit() {
            i += 1;
        }
        had_frac_digits = i > frac_start;
    }

    if !had_int_digits && !had_frac_digits {
        return 0.0;
    }

    // Optional exponent
    let mut end_of_number = i;
    if i < n && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < n && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_digits_start = j;
        while j < n && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_digits_start {
            end_of_number = j;
        }
    } else {
        end_of_number = i;
    }

    let s = match std::str::from_utf8(&bytes[start..end_of_number]) {
        Ok(s) => s,
        Err(_) => return 0.0,
    };
    s.parse::<f64>().unwrap_or(0.0)
}

/// Mirrors C's `(int)` cast from a `double` on x86_64 (cvttsd2si):
/// values outside i32 range (including NaN and infinities) yield 0x80000000.
fn double_to_int_c(x: f64) -> i32 {
    if x.is_nan() {
        return i32::MIN;
    }
    // Truncation toward zero is what `as i32` does for in-range values.
    // For out-of-range values, Rust saturates while C wraps to INT_MIN
    // (using cvttsd2si). Match C's behaviour.
    if x >= 2147483648.0_f64 || x < -2147483648.0_f64 {
        return i32::MIN;
    }
    x as i32
}

fn bad(stdin: &mut impl Read) {
    let mut data: f32 = 0.0;
    {
        if let Some(buf) = c_fgets(stdin, CHAR_ARRAY_SIZE) {
            data = c_atof(&buf) as f32;
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let result = double_to_int_c(100.0 / data as f64);
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = double_to_int_c(100.0 / data as f64);
    print_int_line(result);
}

fn good_b2g(stdin: &mut impl Read) {
    let mut data: f32 = 0.0;
    {
        if let Some(buf) = c_fgets(stdin, CHAR_ARRAY_SIZE) {
            data = c_atof(&buf) as f32;
        } else {
            print_line("fgets() failed.");
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = double_to_int_c(100.0 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good(stdin: &mut impl Read) {
    good_g2b();
    good_b2g(stdin);
}

fn main() {
    let stdin_handle = io::stdin();
    let mut stdin = stdin_handle.lock();

    print_line("Calling good()...");
    good(&mut stdin);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut stdin);
    print_line("Finished bad()");

    let _ = io::stdout().flush();
}
