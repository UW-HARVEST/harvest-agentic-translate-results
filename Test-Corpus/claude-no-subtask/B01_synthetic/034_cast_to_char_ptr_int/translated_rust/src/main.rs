// Translation of c_src/src/main.c to Rust.
// Reproduces byte-identical output for the same inputs.

use std::io::{self, Read, Write};

/// Reads a single byte from stdin (as a u8). Returns None on EOF.
fn read_byte<R: Read>(r: &mut R, peek: &mut Option<u8>) -> Option<u8> {
    if let Some(b) = peek.take() {
        return Some(b);
    }
    let mut buf = [0u8; 1];
    match r.read(&mut buf) {
        Ok(0) => None,
        Ok(_) => Some(buf[0]),
        Err(_) => None,
    }
}

/// Mimic the behavior of `scanf("%d", &x)`. Skips leading whitespace, then
/// reads an optional sign followed by digits. Returns the parsed value, or
/// `None` if no integer could be parsed (in which case the C program leaves
/// `x` unchanged at its initial value of 0).
fn scanf_int<R: Read>(r: &mut R) -> Option<i32> {
    let mut peek: Option<u8> = None;

    // Skip whitespace.
    loop {
        match read_byte(r, &mut peek) {
            None => return None,
            Some(b) => {
                // C's isspace: ' ', '\t', '\n', '\v', '\f', '\r'
                if b == b' ' || b == b'\t' || b == b'\n' || b == 0x0b || b == 0x0c || b == b'\r' {
                    continue;
                } else {
                    peek = Some(b);
                    break;
                }
            }
        }
    }

    // Optional sign.
    let mut negative = false;
    let mut sign_consumed = false;
    if let Some(b) = read_byte(r, &mut peek) {
        if b == b'+' {
            sign_consumed = true;
        } else if b == b'-' {
            negative = true;
            sign_consumed = true;
        } else {
            peek = Some(b);
        }
    } else {
        return None;
    }

    // Digits.
    let mut any_digits = false;
    // Use i64 internally to detect overflow against i32 range.
    // glibc's scanf saturates to INT_MAX/INT_MIN on overflow.
    let mut value: i64 = 0;
    let mut overflowed = false;
    loop {
        match read_byte(r, &mut peek) {
            None => break,
            Some(b) => {
                if b.is_ascii_digit() {
                    any_digits = true;
                    let digit = (b - b'0') as i64;
                    if !overflowed {
                        if negative {
                            value = value.saturating_mul(10).saturating_sub(digit);
                            if value <= i32::MIN as i64 {
                                value = i32::MIN as i64;
                                overflowed = true;
                            }
                        } else {
                            value = value.saturating_mul(10).saturating_add(digit);
                            if value >= i32::MAX as i64 {
                                value = i32::MAX as i64;
                                overflowed = true;
                            }
                        }
                    }
                } else {
                    peek = Some(b);
                    break;
                }
            }
        }
    }

    if !any_digits {
        // No digits consumed: scanf returns 0 and leaves x unchanged.
        // If a sign was consumed, it's effectively put back / discarded;
        // in our simple model we just signal failure.
        let _ = sign_consumed;
        return None;
    }

    Some(value as i32)
}

fn print_hex(p: &[u8], out: &mut impl Write) {
    for byte in p {
        write!(out, "{:02x}", byte).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: i32, out: &mut impl Write) {
    // Reproduce: print_hex((unsigned char *)&x, sizeof(x));
    // sizeof(int) is 4 on virtually all modern targets, and the bytes
    // are in native (little-endian on x86_64) order.
    let bytes = x.to_ne_bytes();
    print_hex(&bytes, out);
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let x = scanf_int(&mut handle).unwrap_or(0);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(x, &mut out);
}
