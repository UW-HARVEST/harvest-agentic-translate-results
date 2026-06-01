use std::io::{self, Read, Write};

/// Mimics C's `scanf("%d", ...)` reading from a byte iterator with one-byte
/// pushback. Returns `Some(value)` on a successful conversion, `None` if no
/// integer could be read (including immediate EOF or non-numeric input after
/// optional whitespace/sign).
fn scanf_int(buf: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (matches isspace in the "C" locale: space, \t,
    // \n, \v, \f, \r).
    while *pos < buf.len() && is_c_whitespace(buf[*pos]) {
        *pos += 1;
    }

    let start = *pos;
    let mut sign: i64 = 1;
    if *pos < buf.len() && (buf[*pos] == b'+' || buf[*pos] == b'-') {
        if buf[*pos] == b'-' {
            sign = -1;
        }
        *pos += 1;
    }

    let digits_start = *pos;
    let mut value: i64 = 0;
    let mut overflow = false;
    while *pos < buf.len() && buf[*pos].is_ascii_digit() {
        let d = (buf[*pos] - b'0') as i64;
        value = value.saturating_mul(10).saturating_add(d);
        if value > i64::from(i32::MAX) + 1 {
            overflow = true;
        }
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits consumed -> conversion failure. scanf "ungets" the
        // non-matching character, but at the top level it's effectively a
        // failure for this assignment.
        *pos = start;
        return None;
    }

    let signed = sign * value;
    let result = if overflow {
        if sign < 0 {
            i32::MIN
        } else {
            i32::MAX
        }
    } else if signed > i64::from(i32::MAX) {
        i32::MAX
    } else if signed < i64::from(i32::MIN) {
        i32::MIN
    } else {
        signed as i32
    };
    Some(result)
}

fn is_c_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let mut pos = 0usize;
    if let Some(v) = scanf_int(&input, &mut pos) {
        x = v;
        if let Some(v2) = scanf_int(&input, &mut pos) {
            y = v2;
        }
    }

    // C's div(x, y): quot truncates toward zero, rem has the sign of x.
    // Rust's i32 `/` and `%` operators have these same semantics.
    let quot = x / y;
    let rem = x % y;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "quotient: {}, remainder: {}", quot, rem);
}
