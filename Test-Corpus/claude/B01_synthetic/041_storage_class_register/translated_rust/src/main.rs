use std::io::Read;

fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    println!("{}", y);
}

/// Read a single integer from stdin in a manner compatible with C's
/// `scanf("%d", &x)`: skip leading whitespace, then read an optional sign
/// followed by decimal digits. Returns `None` if no valid integer can be
/// parsed (matching scanf returning a value other than 1).
fn scanf_int<R: Read>(reader: &mut R) -> Option<i32> {
    let mut byte = [0u8; 1];

    // Skip leading whitespace
    let mut current: Option<u8> = None;
    loop {
        match reader.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => {
                let b = byte[0];
                if !b.is_ascii_whitespace() {
                    current = Some(b);
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    let mut buf = Vec::<u8>::new();
    let first = current.unwrap();

    if first == b'+' || first == b'-' {
        buf.push(first);
        // Need at least one digit afterwards
        match reader.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => {
                if !byte[0].is_ascii_digit() {
                    return None;
                }
                buf.push(byte[0]);
            }
            Err(_) => return None,
        }
    } else if first.is_ascii_digit() {
        buf.push(first);
    } else {
        return None;
    }

    // Read remaining digits
    loop {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0].is_ascii_digit() {
                    buf.push(byte[0]);
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let s = std::str::from_utf8(&buf).ok()?;
    // Mimic C's scanf: it converts to int. If the value overflows the result
    // is undefined; we use wrapping parsing via i64 to be lenient.
    if let Ok(v) = s.parse::<i32>() {
        Some(v)
    } else if let Ok(v) = s.parse::<i64>() {
        Some(v as i32)
    } else {
        None
    }
}

fn main() {
    let mut x: i32 = 0;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    if let Some(v) = scanf_int(&mut handle) {
        x = v;
    }
    driver(x);
}
