// Translated from c_src/src/main.c
// Reproduces C behavior, including buggy behavior in bad().

use std::io::{self, Read};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

const CHAR_ARRAY_SIZE: usize = 20;

/// Read up to `buf_size - 1` bytes from `reader`, stopping after a newline
/// (newline included) or at EOF. Returns `None` if no bytes were read at EOF
/// (mimicking C's fgets returning NULL).
fn fgets<R: Read>(reader: &mut R, buf_size: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    if buf_size == 0 {
        return None;
    }
    while buf.len() + 1 < buf_size {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Mimic C's `atof`: skip leading whitespace, accept an optional sign,
/// digits with optional fractional part, and an optional exponent.
/// Returns 0.0 on parse failure.
fn c_atof(bytes: &[u8]) -> f64 {
    let mut i = 0usize;
    // skip whitespace (C isspace: space, \t, \n, \v, \f, \r)
    while i < bytes.len()
        && matches!(
            bytes[i],
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'
        )
    {
        i += 1;
    }
    let start = i;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let mut has_digits = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digits = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_digits = true;
            i += 1;
        }
    }
    if !has_digits {
        return 0.0;
    }
    let mut end = i;
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let mut exp_has_digits = false;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            exp_has_digits = true;
            j += 1;
        }
        if exp_has_digits {
            end = j;
        }
    }
    let s = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
    s.parse::<f64>().unwrap_or(0.0)
}

/// Mimic the x86 `cvttsd2si` semantics used by typical C compilers when
/// casting a double to int: NaN or out-of-range values yield INT_MIN
/// (the "indefinite integer value"). Otherwise truncate toward zero.
fn double_to_int_c(x: f64) -> i32 {
    if x.is_nan() {
        return i32::MIN;
    }
    let t = x.trunc();
    if t > 2_147_483_647.0_f64 || t < -2_147_483_648.0_f64 {
        return i32::MIN;
    }
    t as i32
}

fn bad<R: Read>(reader: &mut R) {
    let mut data: f32;
    data = 0.0;
    {
        match fgets(reader, CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = c_atof(&input_buffer) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let result = double_to_int_c(100.0_f64 / data as f64);
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = double_to_int_c(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g<R: Read>(reader: &mut R) {
    let mut data: f32;
    data = 0.0;
    {
        match fgets(reader, CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = c_atof(&input_buffer) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = double_to_int_c(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good<R: Read>(reader: &mut R) {
    good_g2b();
    good_b2g(reader);
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    print_line("Calling good()...");
    good(&mut handle);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut handle);
    print_line("Finished bad()");
}
