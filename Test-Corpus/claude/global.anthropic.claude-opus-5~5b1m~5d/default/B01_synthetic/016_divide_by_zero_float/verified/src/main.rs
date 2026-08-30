// Rust translation of c_src/src/main.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{BufRead, BufReader, Stdin, Write};

// ---------------------------------------------------------------------------
// C runtime emulation helpers
// ---------------------------------------------------------------------------

/// Emulates `fgets(buffer, size, stdin)`.
///
/// Reads at most `buffer.len() - 1` bytes, stopping after a newline (which is
/// kept in the buffer) or at end-of-file. A NUL terminator is appended on
/// success. Returns `false` (i.e. NULL in C) when end-of-file is reached
/// before any character is read.
fn fgets<R: BufRead>(buffer: &mut [u8], stream: &mut R) -> bool {
    let capacity = buffer.len();
    if capacity == 0 {
        return false;
    }
    let mut count = 0usize;
    while count + 1 < capacity {
        match read_byte(stream) {
            Some(byte) => {
                buffer[count] = byte;
                count += 1;
                if byte == b'\n' {
                    break;
                }
            }
            None => break,
        }
    }
    if count == 0 {
        // EOF (or error) with no characters read: fgets returns NULL and the
        // buffer contents are left untouched.
        return false;
    }
    buffer[count] = 0;
    true
}

fn read_byte<R: BufRead>(stream: &mut R) -> Option<u8> {
    loop {
        let available = match stream.fill_buf() {
            Ok(slice) => slice,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        if available.is_empty() {
            return None;
        }
        let byte = available[0];
        stream.consume(1);
        return Some(byte);
    }
}

fn is_c_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn starts_with_ci(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len()
        && haystack[..needle.len()]
            .iter()
            .zip(needle.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

/// Emulates `atof()` on a NUL-terminated C string: `strtod()` semantics with
/// the end pointer discarded and no error reporting.
fn c_atof(bytes: &[u8]) -> f64 {
    let text = match bytes.iter().position(|&b| b == 0) {
        Some(nul) => &bytes[..nul],
        None => bytes,
    };
    c_strtod(text)
}

fn c_strtod(s: &[u8]) -> f64 {
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let rest = &s[i..];

    // Infinity
    if starts_with_ci(rest, b"infinity") || starts_with_ci(rest, b"inf") {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // NaN
    if starts_with_ci(rest, b"nan") {
        return if negative { -f64::NAN } else { f64::NAN };
    }

    // Hexadecimal floating point
    if starts_with_ci(rest, b"0x") {
        if let Some(value) = parse_hex_float(&rest[2..]) {
            return if negative { -value } else { value };
        }
        // Only "0" was consumed; the value is zero.
        return if negative { -0.0 } else { 0.0 };
    }

    // Decimal floating point: build the longest valid prefix, then let Rust's
    // correctly-rounded parser produce the value (same as glibc strtod).
    let mut j = 0usize;
    let mut integer_digits = 0usize;
    while j < rest.len() && rest[j].is_ascii_digit() {
        j += 1;
        integer_digits += 1;
    }
    let mut fraction_digits = 0usize;
    let mut significand_end = j;
    if j < rest.len() && rest[j] == b'.' {
        let mut k = j + 1;
        while k < rest.len() && rest[k].is_ascii_digit() {
            k += 1;
            fraction_digits += 1;
        }
        if integer_digits > 0 || fraction_digits > 0 {
            significand_end = k;
            j = k;
        }
    }
    if integer_digits == 0 && fraction_digits == 0 {
        // No conversion could be performed.
        return 0.0;
    }

    let mut number_end = significand_end;
    if j < rest.len() && (rest[j] == b'e' || rest[j] == b'E') {
        let mut k = j + 1;
        if k < rest.len() && (rest[k] == b'+' || rest[k] == b'-') {
            k += 1;
        }
        let exponent_start = k;
        while k < rest.len() && rest[k].is_ascii_digit() {
            k += 1;
        }
        if k > exponent_start {
            number_end = k;
        }
    }

    let literal = String::from_utf8_lossy(&rest[..number_end]).into_owned();
    let magnitude: f64 = literal.parse().unwrap_or(0.0);
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// Parses the part of a hexadecimal float literal that follows `0x`.
/// Returns `None` when there is no hex digit at all (no conversion).
fn parse_hex_float(s: &[u8]) -> Option<f64> {
    let mut i = 0usize;
    let mut mantissa: u128 = 0;
    let mut exponent: i32 = 0;
    let mut digits = 0usize;
    let mut saw_digit = false;

    while i < s.len() && s[i].is_ascii_hexdigit() {
        saw_digit = true;
        let digit = (s[i] as char).to_digit(16).unwrap() as u128;
        if digits < 28 {
            mantissa = mantissa * 16 + digit;
            digits += 1;
        } else {
            exponent += 4;
        }
        i += 1;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && s[i].is_ascii_hexdigit() {
            saw_digit = true;
            let digit = (s[i] as char).to_digit(16).unwrap() as u128;
            if digits < 28 {
                mantissa = mantissa * 16 + digit;
                digits += 1;
                exponent -= 4;
            }
            i += 1;
        }
    }
    if !saw_digit {
        return None;
    }
    if i < s.len() && (s[i] == b'p' || s[i] == b'P') {
        let mut k = i + 1;
        let mut negative_exponent = false;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            negative_exponent = s[k] == b'-';
            k += 1;
        }
        let start = k;
        let mut value: i64 = 0;
        while k < s.len() && s[k].is_ascii_digit() {
            value = (value * 10 + (s[k] - b'0') as i64).min(1 << 30);
            k += 1;
        }
        if k > start {
            exponent = exponent.saturating_add(if negative_exponent {
                -value as i32
            } else {
                value as i32
            });
        }
    }

    Some(ldexp(mantissa as f64, exponent))
}

fn ldexp(mut value: f64, mut exponent: i32) -> f64 {
    while exponent > 1000 {
        value *= 2.0f64.powi(1000);
        exponent -= 1000;
    }
    while exponent < -1000 {
        value *= 2.0f64.powi(-1000);
        exponent += 1000;
    }
    value * 2.0f64.powi(exponent)
}

/// Emulates the x86-64 behaviour of a C `(int)` cast from a floating point
/// value: out-of-range values and NaN yield `INT_MIN` (this is undefined
/// behaviour in C, reproduced here to match the original binary).
fn f64_to_int(value: f64) -> i32 {
    if value.is_nan() {
        return i32::MIN;
    }
    let truncated = value.trunc();
    if truncated >= -2147483648.0f64 && truncated <= 2147483647.0f64 {
        truncated as i32
    } else {
        i32::MIN
    }
}

/// Emulates C's `fabs()` on the double-promoted value.
fn c_fabs(value: f64) -> f64 {
    value.abs()
}

// ---------------------------------------------------------------------------
// Translated program
// ---------------------------------------------------------------------------

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

const CHAR_ARRAY_SIZE: usize = 20;

fn bad(stdin: &mut BufReader<Stdin>) {
    let mut data: f32;
    data = 0.0f32;
    {
        let mut input_buffer = [0u8; CHAR_ARRAY_SIZE];
        if fgets(&mut input_buffer, stdin) {
            data = c_atof(&input_buffer) as f32;
        } else {
            print_line(Some("fgets() failed."));
        }
    }
    {
        let result = f64_to_int(100.0f64 / data as f64);
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0f32;
    {
        let result = f64_to_int(100.0f64 / data as f64);
        print_int_line(result);
    }
}

fn good_b2g(stdin: &mut BufReader<Stdin>) {
    let mut data: f32;
    data = 0.0f32;
    {
        let mut input_buffer = [0u8; CHAR_ARRAY_SIZE];
        if fgets(&mut input_buffer, stdin) {
            data = c_atof(&input_buffer) as f32;
        } else {
            print_line(Some("fgets() failed."));
        }
    }
    if c_fabs(data as f64) > 0.000001f64 {
        let result = f64_to_int(100.0f64 / data as f64);
        print_int_line(result);
    } else {
        print_line(Some("This would result in a divide by zero"));
    }
}

fn good(stdin: &mut BufReader<Stdin>) {
    good_g2b();
    good_b2g(stdin);
}

fn main() {
    let mut stdin = BufReader::new(std::io::stdin());

    print_line(Some("Calling good()..."));
    good(&mut stdin);
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad(&mut stdin);
    print_line(Some("Finished bad()"));

    let _ = std::io::stdout().flush();
}
