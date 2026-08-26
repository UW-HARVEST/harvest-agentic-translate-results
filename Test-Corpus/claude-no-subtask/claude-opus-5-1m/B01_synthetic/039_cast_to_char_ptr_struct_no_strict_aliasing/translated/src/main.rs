use std::io::{self, Read, Write};

#[repr(C)]
#[derive(Default)]
struct House {
    floors: i32,
    bedrooms: i32,
    // 8-byte alignment for f64 means bathrooms starts at offset 8 — already
    // satisfied because (floors + bedrooms) = 8 bytes. Total struct size = 16.
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for byte in p {
        write!(out, "{:02x}", byte).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(floors: i32) {
    let mut house = House::default();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    // Equivalent to: char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));
    let size = std::mem::size_of::<House>();
    let mut raw = vec![0u8; size];
    // SAFETY: House is repr(C) with no internal padding when sized to 16 bytes
    // on the platforms targeted by the original C program. We copy the raw
    // representation, including any padding bytes, exactly as the C program
    // would via memcpy.
    unsafe {
        let src = (&house as *const House) as *const u8;
        std::ptr::copy_nonoverlapping(src, raw.as_mut_ptr(), size);
    }
    print_hex(&raw);
}

/// Mimic C's scanf("%d", &x) for a single int.
///
/// scanf("%d") skips leading whitespace (including newlines), then reads an
/// optional sign followed by decimal digits. Reading stops at the first
/// non-digit character (which is left in the buffer for subsequent reads).
/// If no digits are read, the variable is left untouched (which matches the
/// behavior here since x is initialized to 0).
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0;
    // Skip leading whitespace.
    while i < input.len() && (input[i] as char).is_whitespace() {
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
        // No digits parsed; scanf would not modify the variable.
        return 0;
    }
    let result = sign.wrapping_mul(value);
    result as i32
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).ok();
    let x = scanf_int(&buf);
    driver(x);
}
