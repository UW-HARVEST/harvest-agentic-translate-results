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
//
// Rust translation of c_src/src/main.c (CWE-369: divide by zero driver).
// Behavior, including the C program's undefined/implementation-defined
// behavior on x86-64 (float-to-int conversion of infinity / NaN), is
// reproduced exactly rather than "fixed".

use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// Output helpers (mirror printLine / printIntLine)
// ---------------------------------------------------------------------------

/// `void printLine(const char * line)` -- prints `"%s\n"` for non-NULL input.
/// In this program every call site passes a string literal (never NULL), so
/// the NULL check can never fail; it is kept for structural fidelity.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(line.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

/// `void printIntLine(int intNumber)` -- prints `"%d\n"`.
fn print_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", int_number);
}

// ---------------------------------------------------------------------------
// C runtime emulation
// ---------------------------------------------------------------------------

const CHAR_ARRAY_SIZE: usize = 20;

/// Emulates `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping right after a newline (which is
/// kept in the buffer) or at end of file. Returns `None` (C's NULL) when EOF
/// or an error is hit before any byte was read. Bytes are consumed one at a
/// time from the shared, buffered stdin handle so that a later read continues
/// exactly where this one stopped -- matching C's single `FILE *stdin` stream.
fn fgets(size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() + 1 < size {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn eq_ci_prefix(s: &[u8], pat: &[u8]) -> bool {
    s.len() >= pat.len()
        && s[..pat.len()]
            .iter()
            .zip(pat.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

/// `ldexp(x, n)` (i.e. x * 2^n) without linking libm.
fn ldexp(x: f64, mut n: i32) -> f64 {
    let mut y = x;
    while n > 1023 {
        y *= f64::from_bits(0x7FE0_0000_0000_0000); // 2^1023
        n -= 1023;
        if !y.is_finite() {
            return y;
        }
    }
    while n < -1022 {
        y *= f64::from_bits(0x0010_0000_0000_0000); // 2^-1022
        n += 1022;
        if y == 0.0 {
            return y;
        }
    }
    // 2^n for -1022 <= n <= 1023
    y * f64::from_bits((((n + 1023) as u64) & 0x7FF) << 52)
}

/// Parses a C hexadecimal floating literal body (after the `0x` prefix).
/// Returns the magnitude and the number of bytes consumed, or `None` when the
/// text is not a valid hex float (in which case `strtod` only consumes "0").
fn parse_hex_float(s: &[u8]) -> Option<(f64, usize)> {
    fn hex_val(b: u8) -> Option<u128> {
        match b {
            b'0'..=b'9' => Some((b - b'0') as u128),
            b'a'..=b'f' => Some((b - b'a' + 10) as u128),
            b'A'..=b'F' => Some((b - b'A' + 10) as u128),
            _ => None,
        }
    }

    // Collect the hex digit string, remembering how many followed the point.
    let mut i = 0usize;
    let mut digit_values: Vec<u128> = Vec::new();
    let mut frac_count: i32 = 0;
    while i < s.len() {
        match hex_val(s[i]) {
            Some(d) => {
                digit_values.push(d);
                i += 1;
            }
            None => break,
        }
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() {
            match hex_val(s[i]) {
                Some(d) => {
                    digit_values.push(d);
                    frac_count += 1;
                    i += 1;
                }
                None => break,
            }
        }
    }
    if digit_values.is_empty() {
        return None;
    }

    // value = mantissa * 2^exp
    let mut mantissa: u128 = 0;
    let mut kept = 0usize;
    let mut sticky = false;
    let mut exp: i32 = -4 * frac_count;
    for d in digit_values {
        if mantissa == 0 && d == 0 {
            continue; // leading zeros contribute nothing
        }
        if kept < 28 {
            mantissa = (mantissa << 4) | d;
            kept += 1;
        } else {
            if d != 0 {
                sticky = true;
            }
            exp += 4; // dropped digit only scales the value
        }
    }

    // Optional binary exponent: only consumed when at least one digit follows.
    if i < s.len() && (s[i] == b'p' || s[i] == b'P') {
        let mut j = i + 1;
        let mut neg = false;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            neg = s[j] == b'-';
            j += 1;
        }
        if j < s.len() && s[j].is_ascii_digit() {
            let mut val: i64 = 0;
            while j < s.len() && s[j].is_ascii_digit() {
                if val < 1_000_000 {
                    val = val * 10 + (s[j] - b'0') as i64;
                }
                j += 1;
            }
            if neg {
                val = -val;
            }
            exp = exp.saturating_add(val as i32);
            i = j;
        }
    }

    if mantissa == 0 {
        return Some((0.0, i));
    }
    if sticky {
        mantissa |= 1; // keep a nonzero low bit so rounding sees the remainder
    }
    Some((ldexp(mantissa as f64, exp), i))
}

/// Emulates `atof()` == `strtod(s, NULL)` for the C locale: leading
/// whitespace, optional sign, then a decimal or hexadecimal float, `inf` /
/// `infinity`, or `nan[(chars)]`. Returns 0.0 when no conversion is possible.
fn atof(bytes: &[u8]) -> f64 {
    // atof() operates on a NUL-terminated string.
    let s: &[u8] = match bytes.iter().position(|&b| b == 0) {
        Some(p) => &bytes[..p],
        None => bytes,
    };

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

    // infinity / inf
    if eq_ci_prefix(rest, b"infinity") || eq_ci_prefix(rest, b"inf") {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    // nan / nan(n-char-sequence)
    if eq_ci_prefix(rest, b"nan") {
        return if negative { -f64::NAN } else { f64::NAN };
    }
    // hexadecimal
    if rest.len() >= 2 && rest[0] == b'0' && (rest[1] == b'x' || rest[1] == b'X') {
        if let Some((mag, _used)) = parse_hex_float(&rest[2..]) {
            return if negative { -mag } else { mag };
        }
        // Not a valid hex float: strtod converts just the leading "0".
        return if negative { -0.0 } else { 0.0 };
    }

    // decimal: digits [ '.' digits ] [ ('e'|'E') [sign] digits ]
    let mut j = 0usize;
    let mut int_digits = 0usize;
    while j < rest.len() && rest[j].is_ascii_digit() {
        j += 1;
        int_digits += 1;
    }
    let mut frac_digits = 0usize;
    if j < rest.len() && rest[j] == b'.' {
        j += 1;
        while j < rest.len() && rest[j].is_ascii_digit() {
            j += 1;
            frac_digits += 1;
        }
    }
    if int_digits == 0 && frac_digits == 0 {
        return 0.0; // no conversion performed
    }
    let mut end = j;
    if j < rest.len() && (rest[j] == b'e' || rest[j] == b'E') {
        let mut k = j + 1;
        if k < rest.len() && (rest[k] == b'+' || rest[k] == b'-') {
            k += 1;
        }
        if k < rest.len() && rest[k].is_ascii_digit() {
            while k < rest.len() && rest[k].is_ascii_digit() {
                k += 1;
            }
            end = k;
        }
    }

    let text = match std::str::from_utf8(&rest[..end]) {
        Ok(t) => t,
        Err(_) => return 0.0,
    };
    // Rust's f64 parser is correctly rounded, like glibc's strtod.
    let magnitude: f64 = text.parse().unwrap_or(0.0);
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// Emulates the x86-64 `(int)` cast of a `double` (`cvttsd2si`): values that
/// are NaN or outside the range of `int` yield the "integer indefinite"
/// value `INT_MIN`. This is undefined behavior in C, reproduced here as the
/// original binary behaves.
fn double_to_int(v: f64) -> i32 {
    if v.is_nan() {
        return i32::MIN;
    }
    let t = v.trunc();
    if t >= 2147483648.0 || t < -2147483648.0 {
        return i32::MIN;
    }
    t as i32
}

// ---------------------------------------------------------------------------
// Translated program logic
// ---------------------------------------------------------------------------

fn bad() {
    let mut data: f32;
    data = 0.0f32;
    {
        // char inputBuffer[CHAR_ARRAY_SIZE];
        match fgets(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line(Some("fgets() failed."));
            }
        }
    }
    {
        // FLAW: no check for zero -- 100.0 / 0.0 is a divide by zero.
        let result = double_to_int(100.0f64 / data as f64);
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0f32;
    {
        let result = double_to_int(100.0f64 / data as f64);
        print_int_line(result);
    }
}

fn good_b2g() {
    let mut data: f32;
    data = 0.0f32;
    {
        match fgets(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line(Some("fgets() failed."));
            }
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = double_to_int(100.0f64 / data as f64);
        print_int_line(result);
    } else {
        print_line(Some("This would result in a divide by zero"));
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
    let _ = io::stdout().flush();
    std::process::exit(0);
}
