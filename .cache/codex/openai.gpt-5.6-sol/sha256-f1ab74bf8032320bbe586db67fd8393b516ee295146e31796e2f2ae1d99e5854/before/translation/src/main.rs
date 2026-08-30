use std::io::{self, Read, Write};

fn read_byte(input: &mut impl Read) -> Option<u8> {
    let mut byte = [0];
    match input.read(&mut byte) {
        Ok(1) => Some(byte[0]),
        _ => None,
    }
}

fn scanf_decimal(input: &mut impl Read) -> Option<i32> {
    let mut byte = read_byte(input);
    while matches!(byte, Some(b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')) {
        byte = read_byte(input);
    }

    let negative = match byte {
        Some(b'+') => false,
        Some(b'-') => true,
        _ => {
            if !matches!(byte, Some(b'0'..=b'9')) {
                return None;
            }
            false
        }
    };

    if matches!(byte, Some(b'+' | b'-')) {
        byte = read_byte(input);
    }

    let mut matched = false;
    let mut magnitude = 0_u64;
    while let Some(digit @ b'0'..=b'9') = byte {
        matched = true;
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(u64::from(digit - b'0'));
        byte = read_byte(input);
    }

    matched.then(|| {
        let value = if negative {
            let magnitude = magnitude.min(1_u64 << 63);
            if magnitude == 1_u64 << 63 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else {
            magnitude.min(i64::MAX as u64) as i64
        };
        value as i32
    })
}

fn driver(x: i32) {
    let mut y = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    let _ = writeln!(io::stdout().lock(), "{y}");
}

fn main() {
    let mut x = 0;
    if let Some(parsed) = scanf_decimal(&mut io::stdin().lock()) {
        x = parsed;
    }
    driver(x);
}
