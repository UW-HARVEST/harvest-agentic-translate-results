// Translation of the original C program to Rust producing byte-identical output.
//
// The original C source intentionally exhibits undefined / implementation-defined
// behaviour (signed integer overflow during `data * 2` in `bad()`).  This
// translation reproduces the observed behaviour on common platforms where
// `char` is a signed 8-bit integer and overflow wraps modulo 2^8.

use std::io::{self, Read};

fn print_line(line: &str) {
    // Mirrors printf("%s\n", line) when line != NULL.
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    // In C, `printf("%02x\n", charHex)` triggers default argument promotion
    // (char -> int) and `%x` reads an `unsigned int`.  Negative `char` values
    // become large 32-bit values like 0xFFFFFFFE.  We emulate that here.
    let promoted = char_hex as i32 as u32;
    // %02x prints lowercase hex with at least 2 digits (zero-padded).
    println!("{:02x}", promoted);
}

const CHAR_MAX: i8 = i8::MAX; // 127

fn bad() {
    let data: i8;
    data = CHAR_MAX;
    if data > 0 {
        // `data * 2` in C promotes both operands to int and produces 254;
        // assigning back into a (signed) char wraps to -2 on common platforms.
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8;
    data = 2;
    if data > 0 {
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(result);
    }
}

#[allow(unused_assignments)]
fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = (data as i32 * 2) as i8;
            print_hex_char_line(result);
        } else {
            print_line("data value is too large to perform arithmetic safely.");
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

/// Mimic C's `scanf("%d", &x)`: skip leading whitespace, read an optional sign,
/// then consume decimal digits.  If no digits are read, leave `x` unchanged.
fn scanf_int(input: &mut &[u8]) -> Option<i32> {
    // Skip leading whitespace (space, tab, newline, vertical tab, form feed, carriage return).
    while let Some(&b) = input.first() {
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0b || b == 0x0c {
            *input = &input[1..];
        } else {
            break;
        }
    }

    let mut sign: i32 = 1;
    if let Some(&b) = input.first() {
        if b == b'+' {
            *input = &input[1..];
        } else if b == b'-' {
            sign = -1;
            *input = &input[1..];
        }
    }

    let mut digits_seen = false;
    let mut value: i64 = 0;
    while let Some(&b) = input.first() {
        if b.is_ascii_digit() {
            digits_seen = true;
            value = value
                .saturating_mul(10)
                .saturating_add((b - b'0') as i64);
            *input = &input[1..];
        } else {
            break;
        }
    }

    if !digits_seen {
        return None;
    }

    // Saturate to i32 range as C scanf does on overflow (UB technically, but
    // common implementations clamp).
    let signed_value = if sign < 0 { -value } else { value };
    Some(signed_value as i32)
}

fn main() {
    let mut buf = Vec::new();
    // Read all of stdin (scanf may read across newlines).
    let _ = io::stdin().read_to_end(&mut buf);
    let mut slice: &[u8] = &buf;

    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&mut slice) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
