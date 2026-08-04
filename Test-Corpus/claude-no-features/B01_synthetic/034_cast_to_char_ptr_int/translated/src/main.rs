// Translated from C to Rust. Produces byte-identical output to the original C
// program. The C program reads an int with scanf("%d", &x) and prints the raw
// bytes of `x` as lower-case hex, in memory order (little-endian on typical
// x86_64 / ARM64 systems), followed by a newline.

use std::io::{self, Read, Write, BufWriter};

fn print_hex<W: Write>(out: &mut W, bytes: &[u8]) {
    for b in bytes {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, x: i32) {
    // Equivalent to print_hex((unsigned char *)&x, sizeof(x));
    // On the target architectures (little-endian), this writes the 4 bytes
    // of `x` in little-endian order.
    let bytes = x.to_ne_bytes();
    print_hex(out, &bytes);
}

/// Mimic C's scanf("%d", &x) behavior:
///  * Skip leading whitespace (space, tab, newline, etc.).
///  * Parse an optional '+' or '-' sign.
///  * Parse one or more decimal digits.
///  * Stop at the first non-digit character (which is left in the input).
///  * If no digits are matched, the destination is unchanged. Since the C
///    program initializes `x = 0`, we leave the value at 0 in that case.
fn scanf_d(input: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip whitespace
    while i < input.len() && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= input.len() {
        return 0;
    }
    let mut sign: i64 = 1;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        sign = -1;
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((input[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        // No digits matched; the destination remains its previous value (0).
        return 0;
    }
    let signed = value.wrapping_mul(sign);
    // Truncate to i32 to mirror the int width on typical platforms.
    signed as i32
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).expect("failed to read stdin");

    let x: i32 = scanf_d(&buf);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(&mut out, x);
    out.flush().unwrap();
}
