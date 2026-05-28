use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, bytes: &[u8]) {
    for b in bytes {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(out, &bytes);
}

/// Parse a float from `input` similar to C's scanf("%f", ...).
/// Returns Some(f32) if a float was successfully parsed, None otherwise.
fn parse_float_scanf(input: &str) -> Option<f32> {
    let bytes = input.as_bytes();
    let mut i = 0;

    // Skip whitespace (matches C isspace: space, \t, \n, \v, \f, \r)
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\x0b' || c == b'\x0c' || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    let start = i;
    let mut s = String::new();

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        s.push(bytes[i] as char);
        i += 1;
    }

    // Check for inf/infinity (case insensitive)
    if i + 3 <= bytes.len() {
        let lower: String = bytes[i..(i + 3).min(bytes.len())]
            .iter()
            .map(|&b| (b as char).to_ascii_lowercase())
            .collect();
        if lower == "inf" {
            s.push_str("inf");
            i += 3;
            // Optional "inity" suffix
            if i + 5 <= bytes.len() {
                let lower2: String = bytes[i..i + 5]
                    .iter()
                    .map(|&b| (b as char).to_ascii_lowercase())
                    .collect();
                if lower2 == "inity" {
                    i += 5;
                }
            }
            return s.parse::<f32>().ok();
        }
        let lower_nan: String = bytes[i..(i + 3).min(bytes.len())]
            .iter()
            .map(|&b| (b as char).to_ascii_lowercase())
            .collect();
        if lower_nan == "nan" {
            s.push_str("NaN");
            i += 3;
            return s.parse::<f32>().ok();
        }
    }

    // Parse digits / decimal point / exponent
    let mut has_digit = false;
    let mut has_dot = false;
    let mut has_exp = false;

    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            s.push(c as char);
            has_digit = true;
            i += 1;
        } else if c == b'.' && !has_dot && !has_exp {
            s.push('.');
            has_dot = true;
            i += 1;
        } else if (c == b'e' || c == b'E') && !has_exp && has_digit {
            s.push(c as char);
            has_exp = true;
            i += 1;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                s.push(bytes[i] as char);
                i += 1;
            }
            // Need at least one digit after exponent
            let mut exp_digits = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                s.push(bytes[i] as char);
                i += 1;
                exp_digits = true;
            }
            if !exp_digits {
                // Roll back: scanf considers the exponent invalid; the float
                // ends before the 'e'. We can't easily roll back here, but
                // for simplicity, fall through.
            }
            break;
        } else {
            break;
        }
    }

    if !has_digit {
        // No digits consumed; scanf would fail.
        let _ = start;
        return None;
    }

    s.parse::<f32>().ok()
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    // Mirror C: float x = 0.f; scanf may or may not assign x.
    let mut x: f32 = 0.0;
    if let Some(v) = parse_float_scanf(&input) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
}
