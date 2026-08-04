use std::io::{self, Read};

fn is_scanf_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\t' | b'\r' | 0x0b | 0x0c)
}

fn scan_int(data: &[u8], pos: &mut usize) -> Option<i32> {
    while *pos < data.len() && is_scanf_whitespace(data[*pos]) {
        *pos += 1;
    }

    let start = *pos;
    let mut sign = 1i64;
    if *pos < data.len() {
        match data[*pos] {
            b'+' => *pos += 1,
            b'-' => {
                sign = -1;
                *pos += 1;
            }
            _ => {}
        }
    }

    let digits_start = *pos;
    let mut value = 0i64;
    while *pos < data.len() && data[*pos].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((data[*pos] - b'0') as i64);
        *pos += 1;
    }

    if *pos == digits_start {
        *pos = start;
        None
    } else {
        Some((value.wrapping_mul(sign)) as i32)
    }
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let mut x = 1i32;
    let mut y = 1i32;
    let mut pos = 0usize;

    if let Some(value) = scan_int(&input, &mut pos) {
        x = value;
        if let Some(value) = scan_int(&input, &mut pos) {
            y = value;
        }
    }

    let quotient = x / y;
    let remainder = x % y;
    println!("quotient: {}, remainder: {}", quotient, remainder);
}
