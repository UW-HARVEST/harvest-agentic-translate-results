use std::io::{self, Read, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

/// Read a float from stdin in a manner equivalent to C's scanf("%f", &x):
/// skip leading whitespace (including newlines), then parse the longest
/// prefix that looks like a floating-point number.
fn scanf_float<R: Read>(reader: &mut R) -> Option<f32> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf).ok()?;

    let bytes = buf.as_bytes();
    let mut i = 0;
    // Skip leading whitespace (matching C isspace: space, \t, \n, \r, \v, \f)
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }

    let start = i;

    // Optional sign
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }

    // Check for hex prefix (scanf %f supports 0x/0X hex floats)
    let is_hex = i + 1 < bytes.len()
        && bytes[i] == b'0'
        && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X');

    if is_hex {
        i += 2;
        // hex digits, optional dot, hex digits
        while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
                i += 1;
            }
        }
        // optional binary exponent p/P
        if i < bytes.len() && (bytes[i] == b'p' || bytes[i] == b'P') {
            i += 1;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
        }
    } else {
        // Decimal digits, optional dot, digits, optional e/E exponent
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
        }
        if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
            i += 1;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
        }
    }

    let token = std::str::from_utf8(&bytes[start..i]).ok()?;
    // Try to parse; also handle "inf", "infinity", "nan" case-insensitively.
    if let Ok(v) = token.parse::<f32>() {
        return Some(v);
    }
    let lower = token.to_ascii_lowercase();
    lower.parse::<f32>().ok()
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if let Some(x) = scanf_float(&mut handle) {
        driver(x);
    }
}
