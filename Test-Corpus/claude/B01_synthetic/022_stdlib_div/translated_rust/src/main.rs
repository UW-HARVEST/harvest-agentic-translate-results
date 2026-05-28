use std::io::{self, Read, Write};

/// Read a single byte from stdin. Returns None on EOF.
fn read_byte(stdin: &mut impl Read, peeked: &mut Option<u8>) -> Option<u8> {
    if let Some(b) = peeked.take() {
        return Some(b);
    }
    let mut buf = [0u8; 1];
    match stdin.read(&mut buf) {
        Ok(0) => None,
        Ok(_) => Some(buf[0]),
        Err(_) => None,
    }
}

fn is_c_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Read a signed decimal integer following scanf("%d") semantics.
/// Returns Some(value) if a valid integer was read, None otherwise.
/// Sets `consumed_any` if any non-whitespace character was consumed.
fn scanf_int(
    stdin: &mut impl Read,
    peeked: &mut Option<u8>,
) -> Option<i32> {
    // Skip whitespace
    let mut b = loop {
        match read_byte(stdin, peeked) {
            Some(c) if is_c_whitespace(c) => continue,
            Some(c) => break c,
            None => return None,
        }
    };

    let mut negative = false;
    if b == b'-' {
        negative = true;
        b = match read_byte(stdin, peeked) {
            Some(c) => c,
            None => return None,
        };
    } else if b == b'+' {
        b = match read_byte(stdin, peeked) {
            Some(c) => c,
            None => return None,
        };
    }

    if !b.is_ascii_digit() {
        // Push back the byte
        *peeked = Some(b);
        return None;
    }

    // Use i64 accumulation, wrap to i32 like C with optimization off.
    let mut val: i64 = 0;
    loop {
        if b.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((b - b'0') as i64);
            b = match read_byte(stdin, peeked) {
                Some(c) => c,
                None => {
                    let result = if negative { val.wrapping_neg() } else { val };
                    return Some(result as i32);
                }
            };
        } else {
            *peeked = Some(b);
            let result = if negative { val.wrapping_neg() } else { val };
            return Some(result as i32);
        }
    }
}

fn main() {
    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut peeked: Option<u8> = None;

    // scanf("%d %d", &x, &y)
    if let Some(v) = scanf_int(&mut handle, &mut peeked) {
        x = v;
        if let Some(v2) = scanf_int(&mut handle, &mut peeked) {
            y = v2;
        }
    }

    // div(x, y) - quotient and remainder with C-style truncation toward zero
    // In C, integer division truncates toward zero, and (x/y)*y + x%y == x.
    // Rust's i32 / and % operators also truncate toward zero, matching C99+.
    // Note: division by zero will panic, matching C's undefined behavior (SIGFPE).
    let quot = x / y;
    let rem = x % y;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "quotient: {}, remainder: {}\n", quot, rem);
    let _ = out.flush();
}
