use std::io::{self, Read, Write};

/// Mimics C's scanf("%d", ...).
///
/// Returns Some(value) if a number was successfully parsed.
/// Returns None if EOF or matching failure occurred (in which case the
/// caller should leave the destination variable unchanged, just like scanf).
///
/// Side effect: consumes from `bytes` (advances the index `pos`) up through
/// any leading whitespace and the parsed integer.
fn scanf_i32(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (matches isspace in C locale: space, \t, \n,
    // \v, \f, \r).
    while *pos < bytes.len() {
        let c = bytes[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'\x0b' || c == b'\x0c' {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= bytes.len() {
        return None;
    }

    let start = *pos;
    let mut negative = false;

    // Optional sign
    if bytes[*pos] == b'+' {
        *pos += 1;
    } else if bytes[*pos] == b'-' {
        negative = true;
        *pos += 1;
    }

    let digits_start = *pos;
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits found - matching failure. Per scanf semantics the
        // already-consumed sign character is "putback" conceptually, but
        // since we won't try again, just rewind position to start.
        *pos = start;
        return None;
    }

    // Parse the digits, mimicking C's int (32-bit signed) wraparound on
    // overflow. C's scanf with %d on overflow has undefined behavior; we
    // emulate by using wrapping arithmetic.
    let mut value: i32 = 0;
    for &d in &bytes[digits_start..*pos] {
        let digit = (d - b'0') as i32;
        value = value.wrapping_mul(10);
        if negative {
            value = value.wrapping_sub(digit);
        } else {
            value = value.wrapping_add(digit);
        }
    }

    Some(value)
}

fn driver(x: i32, y: i32, out: &mut impl Write) -> io::Result<()> {
    let result = x | !y;
    write!(out, "{}", result)?;
    // puts("") prints "\n"
    writeln!(out)?;
    Ok(())
}

fn main() -> io::Result<()> {
    // Read all of stdin so we can mimic scanf's behavior of reading across
    // newlines and skipping whitespace.
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    let mut pos = 0usize;
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    if let Some(v) = scanf_i32(&input, &mut pos) {
        x = v;
    }
    if let Some(v) = scanf_i32(&input, &mut pos) {
        y = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(x, y, &mut out)?;
    out.flush()?;
    Ok(())
}
