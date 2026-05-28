use std::io::{self, Read, Write};

fn print_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", int_number);
}

fn bad() {
    // Replicates: data = (int *)alloca(10); int source[10] = {0};
    // Loop copies source[i] to data[i] for i in 0..10. Then prints data[0].
    // In the original C, alloca(10) only allocates 10 bytes (a bug),
    // but we only ever read data[0] which is 0 from source[0].
    let source: [i32; 10] = [0; 10];
    let mut data: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let source: [i32; 10] = [0; 10];
    let mut data: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

/// Read an integer from stdin in a way compatible with C's scanf("%d", ...).
/// Skips leading whitespace (including newlines), then reads an optional sign
/// and consecutive digits. Returns 0 if no integer was successfully read
/// (matches C's behavior where `x` was initialized to 0).
fn scanf_int() -> i32 {
    let mut buf = [0u8; 1];
    let mut stdin = io::stdin();

    // Skip whitespace
    let first: u8 = loop {
        match stdin.read(&mut buf) {
            Ok(0) => return 0,
            Ok(_) => {
                let b = buf[0];
                if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
                    || b == 0x0b || b == 0x0c
                {
                    continue;
                }
                break b;
            }
            Err(_) => return 0,
        }
    };

    let mut sign: i64 = 1;
    let mut have_digit = false;
    let mut value: i64 = 0;

    if first == b'-' {
        sign = -1;
    } else if first == b'+' {
        // positive, no-op
    } else if first.is_ascii_digit() {
        value = (first - b'0') as i64;
        have_digit = true;
    } else {
        // Non-numeric character; scanf would return 0 conversions and leave x unchanged.
        return 0;
    }

    loop {
        match stdin.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                let b = buf[0];
                if b.is_ascii_digit() {
                    value = value
                        .saturating_mul(10)
                        .saturating_add((b - b'0') as i64);
                    have_digit = true;
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if !have_digit {
        return 0;
    }

    let result = sign * value;
    // Truncate to i32 like C does on overflow (implementation-defined but typical).
    result as i32
}

fn main() {
    let x: i32 = scanf_int();

    if x != 0 {
        good();
    } else {
        bad();
    }
}
