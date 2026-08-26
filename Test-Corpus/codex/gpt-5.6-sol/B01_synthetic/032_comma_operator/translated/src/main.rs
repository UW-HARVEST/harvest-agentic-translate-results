use std::io::{self, Read, Write};

fn read_byte<R: Read>(input: &mut R) -> Option<u8> {
    let mut byte = [0_u8; 1];
    loop {
        match input.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => return Some(byte[0]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

fn scan_decimal_int<R: Read>(input: &mut R) -> Option<i32> {
    let first = loop {
        let byte = read_byte(input)?;
        if !matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
            break byte;
        }
    };

    let (negative, mut byte) = match first {
        b'+' => (false, read_byte(input)?),
        b'-' => (true, read_byte(input)?),
        _ => (false, first),
    };

    if !byte.is_ascii_digit() {
        return None;
    }

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude = 0_u64;

    loop {
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'))
            .min(limit);

        match read_byte(input) {
            Some(next) if next.is_ascii_digit() => byte = next,
            _ => break,
        }
    }

    let value = if negative {
        if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        magnitude as i64
    };

    Some(value as i32)
}

fn driver<W: Write>(x: i32, output: &mut W) -> io::Result<()> {
    let mut i = 0_i32;
    let mut j = 0_i32;

    while i < x {
        writeln!(output, "{i} {j}")?;
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }

    Ok(())
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let x = scan_decimal_int(&mut input).unwrap_or(0);

    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    let _ = driver(x, &mut output);
}
