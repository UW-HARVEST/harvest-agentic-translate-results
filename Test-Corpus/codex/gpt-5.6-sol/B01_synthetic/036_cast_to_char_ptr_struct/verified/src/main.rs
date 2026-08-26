use std::io::{self, Read, Write};

fn read_byte(input: &mut impl Read) -> Option<u8> {
    let mut byte = [0_u8; 1];
    input.read_exact(&mut byte).ok().map(|()| byte[0])
}

fn scan_decimal_int(input: &mut impl Read) -> i32 {
    let first = loop {
        match read_byte(input) {
            Some(byte) if byte.is_ascii_whitespace() => {}
            other => break other,
        }
    };

    let (negative, first_digit) = match first {
        Some(b'+') => (false, read_byte(input)),
        Some(b'-') => (true, read_byte(input)),
        other => (false, other),
    };

    let Some(first_digit @ b'0'..=b'9') = first_digit else {
        return 0;
    };

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value = u64::from(first_digit - b'0');

    while let Some(byte) = read_byte(input) {
        let b'0'..=b'9' = byte else {
            break;
        };
        let digit = u64::from(byte - b'0');
        if value > (limit - digit) / 10 {
            value = limit;
        } else {
            value = value * 10 + digit;
        }
    }

    if negative {
        (value as i64).wrapping_neg() as i32
    } else {
        value as i64 as i32
    }
}

fn main() {
    let floors = scan_decimal_int(&mut io::stdin().lock());

    let mut output = String::with_capacity(33);
    for byte in floors
        .to_ne_bytes()
        .into_iter()
        .chain(3_i32.to_ne_bytes())
        .chain(2_f64.to_ne_bytes())
    {
        use std::fmt::Write;
        write!(output, "{byte:02x}").unwrap();
    }
    output.push('\n');
    io::stdout().write_all(output.as_bytes()).unwrap();
}
