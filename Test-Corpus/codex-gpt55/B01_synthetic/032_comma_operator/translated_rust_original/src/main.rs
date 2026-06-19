use std::io::{self, Read, Write};

fn scan_int(input: &[u8]) -> Option<i32> {
    let mut index = 0;

    while index < input.len() && input[index].is_ascii_whitespace() {
        index += 1;
    }

    let sign = if index < input.len() && (input[index] == b'+' || input[index] == b'-') {
        let sign = input[index];
        index += 1;
        sign
    } else {
        b'+'
    };

    if index >= input.len() || !input[index].is_ascii_digit() {
        return None;
    }

    let limit = if sign == b'-' {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value: u64 = 0;
    while index < input.len() && input[index].is_ascii_digit() {
        let digit = (input[index] - b'0') as u64;
        value = value.saturating_mul(10).saturating_add(digit).min(limit);
        index += 1;
    }

    let value = if sign == b'-' {
        if value == limit {
            i64::MIN
        } else {
            -(value as i64)
        }
    } else {
        value as i64
    };

    Some(value as i32)
}

fn driver(x: i32, out: &mut impl Write) -> io::Result<()> {
    let mut i: i32 = 0;
    let mut j: i32 = 0;

    while i < x {
        writeln!(out, "{} {}", i, j)?;
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    let x = scan_int(&input).unwrap_or(0);
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    driver(x, &mut out)
}
