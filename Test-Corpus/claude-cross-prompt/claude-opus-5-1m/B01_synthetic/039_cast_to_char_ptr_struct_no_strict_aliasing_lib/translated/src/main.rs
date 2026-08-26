// Translation of c_src/src/driver.c to Rust.
//
// The C source defines a `driver(int floors)` function that:
//   1. Initializes a `house_t` struct (zeroed) with the given `floors`,
//      `bedrooms = 3`, `bathrooms = 2.0`.
//   2. Memcpys the struct bytes into a raw byte buffer.
//   3. Prints the buffer as lowercase hex followed by a newline.
//
// We add a small driver/main wrapper that reads an integer from stdin
// (mimicking C's scanf("%d") behavior — skipping leading whitespace including
// newlines) and calls `driver`.

use std::io::{self, Read, Write};

#[repr(C)]
#[derive(Copy, Clone)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for byte in p {
        buf.push_str(&format!("{:02x}", byte));
    }
    buf.push('\n');
    out.write_all(buf.as_bytes()).expect("write failed");
}

fn driver(floors: i32) {
    // Zero-initialize, matching the C `{0}` initializer (all padding zero too).
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };

    // Copy struct bytes into a raw buffer of size_of::<House>().
    let size = std::mem::size_of::<House>();
    let mut raw = vec![0u8; size];
    // SAFETY: We're copying `size` bytes from a valid `House` instance into a
    // buffer of exactly that size. This mirrors the C `memcpy(raw, &house, sizeof(house))`.
    unsafe {
        let src = (&house as *const House) as *const u8;
        let dst = raw.as_mut_ptr();
        std::ptr::copy_nonoverlapping(src, dst, size);
    }

    print_hex(&raw);
}

/// Read a single decimal integer from stdin, mimicking C's `scanf("%d", &x)`:
///   - skip leading whitespace (including spaces, tabs, newlines)
///   - read optional sign
///   - read decimal digits until non-digit / EOF
fn scanf_int(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace.
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return None;
    }
    let mut sign: i64 = 1;
    if bytes[*pos] == b'+' {
        *pos += 1;
    } else if bytes[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }
    let start = *pos;
    let mut value: i64 = 0;
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((bytes[*pos] - b'0') as i64);
        *pos += 1;
    }
    if *pos == start {
        return None;
    }
    Some((sign * value) as i32)
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }
    let mut pos = 0usize;
    if let Some(x) = scanf_int(&input, &mut pos) {
        driver(x);
    }
}
