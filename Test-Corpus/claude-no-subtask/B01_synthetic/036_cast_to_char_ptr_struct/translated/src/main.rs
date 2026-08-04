use std::io::{self, Read, Write};

#[repr(C)]
#[derive(Default)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut s = String::with_capacity(bytes.len() * 2 + 1);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    out.write_all(s.as_bytes()).unwrap();
}

fn driver(floors: i32) {
    let mut house = House::default();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let size = std::mem::size_of::<House>();
    let ptr = &house as *const House as *const u8;
    // Safety: reading the bytes of a #[repr(C)] struct that we fully initialized.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, size) };
    print_hex(bytes);
}

/// Mimic C scanf("%d", &x) reading: skip leading whitespace (including newlines),
/// then read an optional sign and digits. If parsing fails, x remains 0.
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0;
    // Skip leading whitespace
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }
    if i >= input.len() {
        return 0;
    }
    let mut sign: i64 = 1;
    if input[i] == b'-' {
        sign = -1;
        i += 1;
    } else if input[i] == b'+' {
        i += 1;
    }
    let start = i;
    let mut val: i64 = 0;
    while i < input.len() {
        let c = input[i];
        if c.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            i += 1;
        } else {
            break;
        }
    }
    if i == start {
        // No digits parsed; scanf would return 0 conversions, x stays 0.
        return 0;
    }
    (val.wrapping_mul(sign)) as i32
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let x = scanf_int(&input);
    driver(x);
}
