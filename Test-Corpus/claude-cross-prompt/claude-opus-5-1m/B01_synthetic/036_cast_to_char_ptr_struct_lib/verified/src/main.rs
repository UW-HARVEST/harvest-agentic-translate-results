// Translated from C to Rust to produce byte-identical output.

use std::io::{self, Read, Write};

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct HouseT {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    out.write_all(s.as_bytes()).unwrap();
    out.flush().unwrap();
}

fn driver(floors: i32) {
    // house_t house = {0};   (zero-init all bytes including any padding)
    let mut house = HouseT::default();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    // Cast struct to byte slice; size_of::<HouseT>() must match C layout.
    let size = std::mem::size_of::<HouseT>();
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts((&house as *const HouseT) as *const u8, size)
    };
    print_hex(bytes);
}

/// Parse an integer from a buffer in a way that mimics C's scanf("%d", ...):
/// - skip leading whitespace (incl. newlines)
/// - optional sign
/// - read consecutive decimal digits
/// Returns Some(value) if at least one digit was parsed.
fn scanf_int(buf: &[u8], pos: &mut usize) -> Option<i32> {
    // skip whitespace
    while *pos < buf.len() && (buf[*pos] as char).is_whitespace() {
        *pos += 1;
    }
    if *pos >= buf.len() {
        return None;
    }
    let mut neg = false;
    if buf[*pos] == b'+' {
        *pos += 1;
    } else if buf[*pos] == b'-' {
        neg = true;
        *pos += 1;
    }
    let start = *pos;
    let mut val: i64 = 0;
    while *pos < buf.len() && buf[*pos].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((buf[*pos] - b'0') as i64);
        *pos += 1;
    }
    if *pos == start {
        return None;
    }
    if neg {
        val = -val;
    }
    Some(val as i32)
}

fn main() {
    // Read all of stdin (mirrors scanf behavior of reading across newlines).
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).ok();

    let mut pos = 0usize;
    if let Some(n) = scanf_int(&input, &mut pos) {
        driver(n);
    } else {
        // No input; call driver(0) as scanf would leave variable uninitialized,
        // but to be safe we just don't call it.
        driver(0);
    }
}
