use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, p: &[u8]) {
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: i32) {
    let raw = x.to_ne_bytes();
    print_hex(out, &raw);
}

/// Mimics C's scanf("%d", &x) behavior:
/// - Skips leading whitespace
/// - Reads optional sign
/// - Reads digit characters
/// - On overflow, saturates to i32::MAX / i32::MIN (matching glibc)
/// - Returns (value, success). On no match, value is unchanged.
fn scanf_int(input: &[u8]) -> (i32, bool) {
    let mut i = 0usize;
    let n = input.len();

    // Skip whitespace
    while i < n {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            i += 1;
        } else {
            break;
        }
    }

    if i >= n {
        return (0, false);
    }

    let mut negative = false;
    if input[i] == b'-' {
        negative = true;
        i += 1;
    } else if input[i] == b'+' {
        i += 1;
    }

    if i >= n || !input[i].is_ascii_digit() {
        return (0, false);
    }

    // Accumulate digits using i64 then saturate
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < n && input[i].is_ascii_digit() {
        let d = (input[i] - b'0') as i64;
        if !overflow {
            value = value.saturating_mul(10).saturating_add(d);
            if negative {
                if -value < i32::MIN as i64 {
                    overflow = true;
                }
            } else {
                if value > i32::MAX as i64 {
                    overflow = true;
                }
            }
        }
        i += 1;
    }

    let result: i32 = if negative {
        if overflow {
            i32::MIN
        } else {
            (-value) as i32
        }
    } else {
        if overflow {
            i32::MAX
        } else {
            value as i32
        }
    };

    (result, true)
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();

    let mut x: i32 = 0;
    let (parsed, ok) = scanf_int(&buf);
    if ok {
        x = parsed;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
    out.flush().unwrap();
}
