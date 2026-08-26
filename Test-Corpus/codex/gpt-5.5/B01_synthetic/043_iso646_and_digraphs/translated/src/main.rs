use std::io::{self, Read, Write};

fn c_isspace(byte: u8) -> bool {
    matches!(byte, b' ' | 0x0c | b'\n' | b'\r' | b'\t' | 0x0b)
}

fn scanf_d(input: &[u8], pos: &mut usize, target: &mut i32) {
    let len = input.len();

    while *pos < len && c_isspace(input[*pos]) {
        *pos += 1;
    }

    let mut idx = *pos;
    let mut negative = false;

    if idx < len && (input[idx] == b'+' || input[idx] == b'-') {
        negative = input[idx] == b'-';
        idx += 1;
    }

    if idx >= len || !input[idx].is_ascii_digit() {
        return;
    }

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value: u64 = 0;
    while idx < len && input[idx].is_ascii_digit() {
        let digit = (input[idx] - b'0') as u64;
        value = value.saturating_mul(10).saturating_add(digit).min(limit);
        idx += 1;
    }

    *pos = idx;

    let signed = if negative && value == (i64::MAX as u64) + 1 {
        i64::MIN
    } else if negative {
        -(value as i64)
    } else {
        value as i64
    };
    *target = signed as i32;
}

fn driver(x: i32, y: i32) -> i32 {
    x | !y
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let mut pos = 0usize;
    let mut x = 0i32;
    let mut y = 0i32;

    scanf_d(&input, &mut pos, &mut x);
    scanf_d(&input, &mut pos, &mut y);

    let result = driver(x, y);
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{result}").unwrap();
}
