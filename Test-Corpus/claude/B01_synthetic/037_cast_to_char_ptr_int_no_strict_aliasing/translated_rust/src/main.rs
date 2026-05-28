use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(w: &mut W, p: &[u8]) {
    for b in p {
        write!(w, "{:02x}", b).unwrap();
    }
    writeln!(w).unwrap();
}

fn driver<W: Write>(w: &mut W, x: i32) {
    // sizeof(int) == 4 on the target platform; memcpy bytes of x and print hex
    let raw = x.to_ne_bytes();
    print_hex(w, &raw);
}

/// Mimic C's `scanf("%d", &x)` behavior.
/// Returns the parsed integer, or leaves x unchanged (0 here) if no conversion
/// was performed. Reads from `bytes` starting at `pos` and updates `pos`.
fn scanf_d(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (matches C isspace: space, \t, \n, \v, \f, \r)
    while *pos < bytes.len() {
        let c = bytes[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= bytes.len() {
        return None;
    }

    let start = *pos;
    let mut sign: i64 = 1;
    if bytes[*pos] == b'+' {
        *pos += 1;
    } else if bytes[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }

    let digits_start = *pos;
    let mut value: i64 = 0;
    let mut overflow = false;
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        let d = (bytes[*pos] - b'0') as i64;
        value = value.wrapping_mul(10).wrapping_add(d);
        if value > i32::MAX as i64 + 1 {
            overflow = true;
        }
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits consumed: rewind and signal no conversion
        *pos = start;
        return None;
    }

    let signed = sign.wrapping_mul(value);
    let result = if overflow {
        if sign > 0 { i32::MAX } else { i32::MIN }
    } else if signed > i32::MAX as i64 {
        i32::MAX
    } else if signed < i32::MIN as i64 {
        i32::MIN
    } else {
        signed as i32
    };
    Some(result)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).expect("failed to read stdin");

    let mut pos = 0usize;
    let mut x: i32 = 0;
    if let Some(parsed) = scanf_d(&input, &mut pos) {
        x = parsed;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
    out.flush().unwrap();
}
