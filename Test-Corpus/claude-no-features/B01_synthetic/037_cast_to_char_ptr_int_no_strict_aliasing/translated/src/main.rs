use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, p: &[u8]) {
    for &b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: i32) {
    // Equivalent to: char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    // On the typical target (little-endian, sizeof(int) == 4) this is the
    // little-endian representation of x.
    let raw = x.to_ne_bytes();
    print_hex(out, &raw);
}

/// Reads an int from stdin in a manner compatible with C's scanf("%d", &x).
/// Returns Some(value) on a successful conversion, None on matching failure
/// (in which case the caller leaves the destination unchanged).
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace.
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b /* \v */ || c == 0x0c /* \f */
        {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= input.len() {
        return None;
    }

    let start = *pos;
    let mut negative = false;
    if input[*pos] == b'+' {
        *pos += 1;
    } else if input[*pos] == b'-' {
        negative = true;
        *pos += 1;
    }

    let digits_start = *pos;
    let mut value: i64 = 0;
    let mut overflow = false;
    while *pos < input.len() {
        let c = input[*pos];
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i64;
        if !overflow {
            value = value.saturating_mul(10).saturating_add(d);
            if negative {
                if -value < i32::MIN as i64 {
                    overflow = true;
                }
            } else if value > i32::MAX as i64 {
                overflow = true;
            }
        }
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits read; matching failure. Roll back position to start of
        // attempted conversion (scanf semantics: pushback of any sign).
        *pos = start;
        return None;
    }

    let result: i32 = if overflow {
        if negative { i32::MIN } else { i32::MAX }
    } else if negative {
        // value fits in i32 because of the overflow check above
        (-value) as i32
    } else {
        value as i32
    };

    Some(result)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    let mut pos = 0usize;
    if let Some(v) = scanf_int(&input, &mut pos) {
        x = v;
    }

    driver(&mut out, x);
}
