// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to preserve byte-identical behavior with the C original.

use std::io::{self, Read, Write};

// CHAR_MAX in C is 127 when char is signed (the typical platform).
const CHAR_MAX: i8 = i8::MAX;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // C: printf("%s\n", line);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.write_all(b"\n");
    }
}

fn print_hex_char_line(char_hex: i8) {
    // C: printf("%02x\n", charHex);
    // In C, `char` is promoted to `int` (sign-extended for signed char) before
    // being passed as a variadic argument. The `%x` conversion then interprets
    // that argument as `unsigned int`. So for negative char values, the output
    // is the 32-bit two's-complement representation in hex (e.g. -2 -> "fffffffe").
    // The "02" specifier is a minimum field width, so larger values are not truncated.
    let as_int: i32 = char_hex as i32;
    let as_uint: u32 = as_int as u32;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if as_uint <= 0xff {
        // Width 2 minimum padding with leading zeroes, lowercase hex.
        let _ = write!(handle, "{:02x}\n", as_uint);
    } else {
        // For values that don't fit in 2 hex digits, %02x just prints the full value.
        let _ = write!(handle, "{:x}\n", as_uint);
    }
}

fn bad() {
    let data: i8;
    data = CHAR_MAX;
    if data > 0 {
        // In C: `char result = data * 2;`
        // `data` (i8) is promoted to int (i32) for arithmetic: 127 * 2 = 254.
        // The result is then converted back to `char`. Conversion of an
        // out-of-range value to a signed integer is implementation-defined
        // in C, but on the typical (two's-complement, wrapping) platforms
        // this wraps: 254 -> -2.
        let promoted: i32 = (data as i32) * 2;
        let result: i8 = promoted as i8; // truncating cast = wrap to i8
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8;
    data = 2;
    if data > 0 {
        let promoted: i32 = (data as i32) * 2;
        let result: i8 = promoted as i8;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    let _ = data; // first assignment is dead, mirroring the C source
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let promoted: i32 = (data as i32) * 2;
            let result: i8 = promoted as i8;
            print_hex_char_line(result);
        } else {
            print_line(Some("data value is too large to perform arithmetic safely."));
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

/// Emulate `scanf("%d", &x)` against an in-memory buffer.
///
/// scanf with `%d`:
///   - skips any leading whitespace (including newlines),
///   - then reads an optional sign and decimal digits,
///   - stops at the first non-matching character (which is left in the stream).
///
/// On a successful match, returns the parsed value. On no match (EOF, no
/// digits, etc.) returns `None`, in which case the C code leaves `x` unchanged
/// at its initial value of 0.
fn scanf_int(buf: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace.
    while *pos < buf.len() {
        let c = buf[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= buf.len() {
        return None;
    }

    let start = *pos;
    let mut have_sign = false;
    if buf[*pos] == b'+' || buf[*pos] == b'-' {
        have_sign = true;
        *pos += 1;
    }

    let digits_start = *pos;
    while *pos < buf.len() && buf[*pos].is_ascii_digit() {
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits matched; scanf fails. Roll position back to before the
        // sign too — scanf would put back the unmatched sign character.
        *pos = start;
        // (We also bail out if only whitespace was found.)
        let _ = have_sign;
        return None;
    }

    // Parse the matched substring. C's scanf with %d performs no overflow
    // detection beyond UB; in practice glibc clamps. For the inputs this
    // program is exercised with this is irrelevant. We use saturating-style
    // wrapping to keep behaviour deterministic.
    let s = std::str::from_utf8(&buf[start..*pos]).ok()?;
    match s.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => None,
    }
}

fn main() {
    // Slurp all of stdin so we can parse with scanf-like semantics. C's
    // `scanf` happily reads across newlines for `%d`, which `read_to_string`
    // followed by manual parsing replicates here.
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let bytes = input.as_bytes();
    let mut pos = 0usize;

    let mut x: i32 = 0;
    if let Some(v) = scanf_int(bytes, &mut pos) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    // Ensure output is flushed before the process exits.
    let _ = io::stdout().flush();
}
