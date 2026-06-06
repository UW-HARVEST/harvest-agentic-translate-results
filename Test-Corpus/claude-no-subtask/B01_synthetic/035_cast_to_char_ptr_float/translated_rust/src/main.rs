use std::io::{self, Read, Write, BufWriter};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

/// Mimic C's scanf("%f", &x): skip leading whitespace, then read characters
/// matching a float and parse as f32. If parsing fails, x remains unchanged.
fn scan_float(input: &[u8]) -> Option<f32> {
    let mut i = 0;
    // Skip leading whitespace (matching C isspace: space, tab, newline, vtab, ff, cr)
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'\x0b' || c == b'\x0c' {
            i += 1;
        } else {
            break;
        }
    }

    let start = i;

    // Optional sign
    if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
        i += 1;
    }

    // Check for hex prefix
    let is_hex = i + 1 < input.len() && input[i] == b'0' && (input[i + 1] == b'x' || input[i + 1] == b'X');
    if is_hex {
        i += 2;
        // Hex digits
        while i < input.len() && input[i].is_ascii_hexdigit() {
            i += 1;
        }
        if i < input.len() && input[i] == b'.' {
            i += 1;
            while i < input.len() && input[i].is_ascii_hexdigit() {
                i += 1;
            }
        }
        // Binary exponent (required for hex floats but be lenient)
        if i < input.len() && (input[i] == b'p' || input[i] == b'P') {
            i += 1;
            if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
                i += 1;
            }
            while i < input.len() && input[i].is_ascii_digit() {
                i += 1;
            }
        }
    } else {
        // Check for inf/infinity/nan (case insensitive)
        let rest = &input[i..];
        let lower: Vec<u8> = rest.iter().take(8).map(|c| c.to_ascii_lowercase()).collect();
        if lower.starts_with(b"infinity") {
            i += 8;
        } else if lower.starts_with(b"inf") {
            i += 3;
        } else if lower.starts_with(b"nan") {
            i += 3;
            // Optional ( ... )
            if i < input.len() && input[i] == b'(' {
                let mut j = i + 1;
                while j < input.len() && input[j] != b')' {
                    j += 1;
                }
                if j < input.len() && input[j] == b')' {
                    i = j + 1;
                }
            }
        } else {
            // Decimal digits
            while i < input.len() && input[i].is_ascii_digit() {
                i += 1;
            }
            if i < input.len() && input[i] == b'.' {
                i += 1;
                while i < input.len() && input[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Exponent
            if i < input.len() && (input[i] == b'e' || input[i] == b'E') {
                let save = i;
                i += 1;
                if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
                    i += 1;
                }
                let exp_start = i;
                while i < input.len() && input[i].is_ascii_digit() {
                    i += 1;
                }
                if i == exp_start {
                    // No digits after exponent — back off
                    i = save;
                }
            }
        }
    }

    if i == start {
        return None;
    }

    let s = std::str::from_utf8(&input[start..i]).ok()?;
    s.parse::<f32>().ok()
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).ok();

    let mut x: f32 = 0.0;
    if let Some(v) = scan_float(&buf) {
        x = v;
    }
    driver(x);
}
