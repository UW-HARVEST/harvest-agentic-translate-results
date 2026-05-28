use std::io::{self, Read, Write};

const CHAR_MAX: i8 = i8::MAX;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    // C: printf("%02x\n", charHex);
    // In C, signed char is promoted to int via vararg promotion (sign-extended),
    // then %x interprets it as unsigned int.
    let promoted = char_hex as i32;
    let as_unsigned = promoted as u32;
    println!("{:02x}", as_unsigned);
}

fn bad() {
    let data: i8;
    data = CHAR_MAX;
    if data > 0 {
        // C: char result = data * 2;
        // data * 2 is promoted to int, then truncated to char.
        let result = (data as i32).wrapping_mul(2) as i8;
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8;
    data = 2;
    if data > 0 {
        let result = (data as i32).wrapping_mul(2) as i8;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result = (data as i32).wrapping_mul(2) as i8;
            print_hex_char_line(result);
        } else {
            print_line("data value is too large to perform arithmetic safely.");
        }
    }
    let _ = data;
}

fn good() {
    good_g2b();
    good_b2g();
}

/// Read an integer from stdin, mimicking C's scanf("%d", &x) behavior.
/// Skips leading whitespace (including newlines), parses an optional sign and digits.
/// If parsing fails, x remains 0 (its initialized value).
fn scanf_int(input: &mut impl Iterator<Item = u8>, peeked: &mut Option<u8>) -> Option<i32> {
    // Skip whitespace
    let mut c = match peeked.take().or_else(|| input.next()) {
        Some(b) => b,
        None => return None,
    };
    while c.is_ascii_whitespace() {
        c = match input.next() {
            Some(b) => b,
            None => return None,
        };
    }

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        c = match input.next() {
            Some(b) => b,
            None => {
                // no digits read; per scanf semantics, this is a matching failure
                return None;
            }
        };
    }

    if !c.is_ascii_digit() {
        // No digits; matching failure -> x unchanged
        *peeked = Some(c);
        return None;
    }

    let mut val: i64 = 0;
    loop {
        val = val.wrapping_mul(10).wrapping_add((c - b'0') as i64);
        match input.next() {
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                *peeked = Some(b);
                break;
            }
            None => break,
        }
    }

    let result = if negative { -val } else { val };
    Some(result as i32)
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();

    let mut iter = buf.into_iter();
    let mut peeked: Option<u8> = None;

    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&mut iter, &mut peeked) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    io::stdout().flush().unwrap();
}
