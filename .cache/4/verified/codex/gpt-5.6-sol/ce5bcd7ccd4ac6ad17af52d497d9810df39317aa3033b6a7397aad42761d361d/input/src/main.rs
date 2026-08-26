use std::io::{self, Read, Write};

fn read_byte<R: Read>(reader: &mut R) -> Option<u8> {
    let mut byte = [0_u8; 1];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => return Some(byte[0]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn scan_decimal_int<R: Read>(reader: &mut R) -> Option<i32> {
    let mut byte = loop {
        let byte = read_byte(reader)?;
        if !is_c_whitespace(byte) {
            break byte;
        }
    };

    let negative = match byte {
        b'-' => {
            byte = read_byte(reader)?;
            true
        }
        b'+' => {
            byte = read_byte(reader)?;
            false
        }
        _ => false,
    };

    if !byte.is_ascii_digit() {
        return None;
    }

    // glibc scanf parses through a signed long, saturates it on range
    // errors, and then stores the low int bits.
    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude = 0_u64;

    loop {
        let digit = u64::from(byte - b'0');
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|value| value.checked_add(digit))
            .unwrap_or(limit)
            .min(limit);

        match read_byte(reader) {
            Some(next) if next.is_ascii_digit() => byte = next,
            _ => break,
        }
    }

    let parsed = if negative {
        if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        magnitude as i64
    };

    Some(parsed as i32)
}

fn print_line<W: Write>(writer: &mut W, line: Option<&str>) {
    if let Some(line) = line {
        let _ = writeln!(writer, "{line}");
    }
}

fn print_hex_char_line<W: Write>(writer: &mut W, char_hex: i8) {
    let promoted = (i32::from(char_hex)) as u32;
    let _ = writeln!(writer, "{promoted:02x}");
}

fn bad<W: Write>(writer: &mut W) {
    let data = i8::MAX;
    if data > 0 {
        let result = data.wrapping_mul(2);
        print_hex_char_line(writer, result);
    }
}

fn good_g2b<W: Write>(writer: &mut W) {
    let data = 2_i8;
    if data > 0 {
        let result = data.wrapping_mul(2);
        print_hex_char_line(writer, result);
    }
}

fn good_b2g<W: Write>(writer: &mut W) {
    let data = i8::MAX;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result = data.wrapping_mul(2);
            print_hex_char_line(writer, result);
        } else {
            print_line(
                writer,
                Some("data value is too large to perform arithmetic safely."),
            );
        }
    }
}

fn good<W: Write>(writer: &mut W) {
    good_g2b(writer);
    good_b2g(writer);
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let x = scan_decimal_int(&mut input).unwrap_or(0);

    let stdout = io::stdout();
    let mut output = stdout.lock();
    if x != 0 {
        good(&mut output);
    } else {
        bad(&mut output);
    }
}

#[cfg(test)]
mod tests {
    use super::scan_decimal_int;

    #[test]
    fn scans_like_decimal_scanf_for_observable_cases() {
        assert_eq!(scan_decimal_int(&mut &b"\n\t-42 rest"[..]), Some(-42));
        assert_eq!(scan_decimal_int(&mut &b"+7"[..]), Some(7));
        assert_eq!(scan_decimal_int(&mut &b"no number"[..]), None);
        assert_eq!(scan_decimal_int(&mut &b"-"[..]), None);
    }

    #[test]
    fn narrows_the_saturated_signed_long_like_glibc() {
        assert_eq!(scan_decimal_int(&mut &b"4294967296"[..]), Some(0));
        assert_eq!(
            scan_decimal_int(&mut &b"999999999999999999999999"[..]),
            Some(-1)
        );
        assert_eq!(
            scan_decimal_int(&mut &b"-999999999999999999999999"[..]),
            Some(0)
        );
    }
}
