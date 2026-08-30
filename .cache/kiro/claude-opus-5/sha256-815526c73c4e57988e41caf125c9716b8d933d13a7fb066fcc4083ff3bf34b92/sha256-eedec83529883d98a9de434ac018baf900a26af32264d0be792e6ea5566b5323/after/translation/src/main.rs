// Rust translation of c_src/src/main.c
//
// Original C code: Copyright 2025 MIT Lincoln Laboratory (MIT license, see c_src).
//
// The original program demonstrates a divide-by-zero defect (CWE-369). The
// faulty behavior is reproduced here verbatim: `bad()` divides by a value read
// from stdin without checking it for zero, and the resulting infinite /
// out-of-range double is converted to `int`. On x86-64 that conversion is
// implemented by `cvttsd2si`, which yields the "integer indefinite" value
// 0x80000000 (i32::MIN) for NaN, infinities and out-of-range values. That is
// what `c_double_to_int` reproduces below, so the output stays byte-identical.

use std::io::{self, BufRead, Write};

const CHAR_ARRAY_SIZE: usize = 20;

// ---------------------------------------------------------------------------
// Output helpers (printf("%s\n") / printf("%d\n"))
// ---------------------------------------------------------------------------

fn print_line(out: &mut dyn Write, line: &str) {
    // The C version skips printing when the pointer is NULL; every call site
    // passes a string literal, so the message is always printed.
    let _ = write!(out, "{}\n", line);
}

fn print_int_line(out: &mut dyn Write, int_number: i32) {
    let _ = write!(out, "{}\n", int_number);
}

// ---------------------------------------------------------------------------
// stdin: fgets() semantics
// ---------------------------------------------------------------------------

/// Reads at most `size - 1` bytes, stopping after a newline (which is kept in
/// the returned buffer). Returns `None` when no byte could be read (EOF or
/// error), matching `fgets()` returning NULL. Unlike `scanf`, this never reads
/// past the terminating newline.
fn fgets(input: &mut dyn BufRead, size: usize) -> Option<Vec<u8>> {
    if size <= 1 {
        return None;
    }
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < size - 1 {
        match input.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None, // fgets() reports NULL on a read error
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

// ---------------------------------------------------------------------------
// atof() / strtod() semantics
// ---------------------------------------------------------------------------

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn hex_val(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a') as u32 + 10),
        b'A'..=b'F' => Some((b - b'A') as u32 + 10),
        _ => None,
    }
}

fn matches_ci(s: &[u8], at: usize, word: &[u8]) -> bool {
    if s.len() < at + word.len() {
        return false;
    }
    s[at..at + word.len()]
        .iter()
        .zip(word.iter())
        .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

/// `atof(buf)`: the C string ends at the first NUL byte that `fgets` stored (or
/// at the end of the data it read), and the value of the longest parseable
/// prefix is returned. Unparseable input yields 0.0.
fn c_atof(buf: &[u8]) -> f64 {
    let s = match buf.iter().position(|&b| b == 0) {
        Some(i) => &buf[..i],
        None => buf,
    };
    c_strtod(s)
}

fn c_strtod(s: &[u8]) -> f64 {
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    // "infinity" / "inf" / "nan[(chars)]", case insensitive.
    if matches_ci(s, i, b"infinity") || matches_ci(s, i, b"inf") {
        return if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    if matches_ci(s, i, b"nan") {
        return if neg { -f64::NAN } else { f64::NAN };
    }

    // Hexadecimal floating point: 0x1.8p3
    if i + 1 < s.len() && s[i] == b'0' && (s[i + 1] == b'x' || s[i + 1] == b'X') {
        match parse_hex_float(&s[i + 2..]) {
            Some(v) => return if neg { -v } else { v },
            // No hex digits after the prefix: only the leading "0" converts.
            None => return if neg { -0.0 } else { 0.0 },
        }
    }

    // Decimal form: digits with an optional point and an optional exponent.
    let start = i;
    let mut digits = 0usize;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    let mut end = i;
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        let mut j = i + 1;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            j += 1;
        }
        if j < s.len() && s[j].is_ascii_digit() {
            while j < s.len() && s[j].is_ascii_digit() {
                j += 1;
            }
            end = j;
        }
    }

    // The accepted prefix is plain ASCII and uses a grammar that Rust's
    // correctly-rounded parser accepts, so rounding matches strtod().
    let text = match std::str::from_utf8(&s[start..end]) {
        Ok(t) => t,
        Err(_) => return 0.0,
    };
    let v: f64 = text.parse().unwrap_or(0.0);
    if neg {
        -v
    } else {
        v
    }
}

/// Parses the part after `0x`. Returns `None` if there is no hex digit.
fn parse_hex_float(s: &[u8]) -> Option<f64> {
    let mut mantissa: u128 = 0;
    let mut dropped_shift: i64 = 0;
    let mut sticky = false;
    let mut n_digits = 0usize;
    let mut frac_digits: i64 = 0;
    let mut seen_point = false;
    let mut i = 0usize;

    while i < s.len() {
        if let Some(d) = hex_val(s[i]) {
            n_digits += 1;
            if seen_point {
                frac_digits += 1;
            }
            if mantissa < (1u128 << 124) {
                mantissa = mantissa * 16 + d as u128;
            } else {
                dropped_shift += 4;
                if d != 0 {
                    sticky = true;
                }
            }
            i += 1;
        } else if s[i] == b'.' && !seen_point {
            seen_point = true;
            i += 1;
        } else {
            break;
        }
    }
    if n_digits == 0 {
        return None;
    }

    let mut p_exp: i64 = 0;
    if i < s.len() && (s[i] == b'p' || s[i] == b'P') {
        let mut j = i + 1;
        let mut p_neg = false;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            p_neg = s[j] == b'-';
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
            p_exp = if p_neg { -val } else { val };
        }
    }

    let exp2 = p_exp - 4 * frac_digits + dropped_shift;
    Some(scale_to_f64(mantissa, sticky, exp2))
}

/// Rounds `mantissa * 2^exp2` (plus a non-zero tail when `sticky`) to the
/// nearest f64, ties to even.
fn scale_to_f64(mantissa: u128, sticky: bool, exp2: i64) -> f64 {
    if mantissa == 0 {
        return 0.0;
    }
    let high_bit = 127 - mantissa.leading_zeros() as i64;
    let normalized_exp = high_bit + exp2;
    let mut q = normalized_exp - 52;
    if q < -1074 {
        q = -1074; // subnormal range
    }
    let shift = q - exp2;

    let mut n: u128 = if shift <= 0 {
        let sh = (-shift) as u32;
        if sh >= 128 {
            return f64::INFINITY;
        }
        mantissa << sh
    } else if shift >= 128 {
        0
    } else {
        let sh = shift as u32;
        let kept = mantissa >> sh;
        let rem = mantissa & ((1u128 << sh) - 1);
        let half = 1u128 << (sh - 1);
        if rem > half || (rem == half && (sticky || (kept & 1) == 1)) {
            kept + 1
        } else {
            kept
        }
    };

    if n == 0 {
        return 0.0;
    }
    if n >= (1u128 << 53) {
        n >>= 1;
        q += 1;
    }
    if q == -1074 && n < (1u128 << 52) {
        return f64::from_bits(n as u64); // subnormal
    }
    let biased = q + 52 + 1023;
    if biased >= 2047 {
        return f64::INFINITY;
    }
    if biased <= 0 {
        return 0.0;
    }
    let bits = ((biased as u64) << 52) | ((n as u64) - (1u64 << 52));
    f64::from_bits(bits)
}

// ---------------------------------------------------------------------------
// (int) cast of a double, as x86-64 cvttsd2si performs it
// ---------------------------------------------------------------------------

fn c_double_to_int(v: f64) -> i32 {
    let t = v.trunc();
    if t.is_nan() || t < -2147483648.0 || t > 2147483647.0 {
        i32::MIN
    } else {
        t as i32
    }
}

// ---------------------------------------------------------------------------
// Translated program
// ---------------------------------------------------------------------------

fn bad(input: &mut dyn BufRead, out: &mut dyn Write) {
    let mut data: f32 = 0.0;
    {
        match fgets(input, CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = c_atof(&input_buffer) as f32;
            }
            None => {
                print_line(out, "fgets() failed.");
            }
        }
    }
    {
        // No zero check: this is the defect being demonstrated.
        let result = c_double_to_int(100.0f64 / data as f64);
        print_int_line(out, result);
    }
}

fn good_g2b(out: &mut dyn Write) {
    let data: f32 = 2.0;
    {
        let result = c_double_to_int(100.0f64 / data as f64);
        print_int_line(out, result);
    }
}

fn good_b2g(input: &mut dyn BufRead, out: &mut dyn Write) {
    let mut data: f32 = 0.0;
    {
        match fgets(input, CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = c_atof(&input_buffer) as f32;
            }
            None => {
                print_line(out, "fgets() failed.");
            }
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = c_double_to_int(100.0f64 / data as f64);
        print_int_line(out, result);
    } else {
        print_line(out, "This would result in a divide by zero");
    }
}

fn good(input: &mut dyn BufRead, out: &mut dyn Write) {
    good_g2b(out);
    good_b2g(input, out);
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    print_line(&mut out, "Calling good()...");
    good(&mut input, &mut out);
    print_line(&mut out, "Finished good()");
    print_line(&mut out, "Calling bad()...");
    bad(&mut input, &mut out);
    print_line(&mut out, "Finished bad()");

    let _ = out.flush();
}
