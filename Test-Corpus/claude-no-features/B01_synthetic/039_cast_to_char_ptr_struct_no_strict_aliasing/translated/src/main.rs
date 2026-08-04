use std::io::{self, Read, Write};

#[repr(C)]
#[derive(Default)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut out = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        out.push_str(&format!("{:02x}", b));
    }
    out.push('\n');
    handle.write_all(out.as_bytes()).unwrap();
}

fn driver(floors: i32) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    // memcpy struct to a raw byte buffer (mirrors C: char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));)
    let size = std::mem::size_of::<House>();
    let mut raw: Vec<u8> = vec![0u8; size];
    unsafe {
        let src = &house as *const House as *const u8;
        std::ptr::copy_nonoverlapping(src, raw.as_mut_ptr(), size);
    }
    print_hex(&raw);
}

/// Mimic scanf("%d", &x): skip whitespace then read an optional sign + decimal digits.
/// Returns parsed value, or 0 if no input was found (matches C's behavior of leaving x = 0
/// when scanf fails to match).
fn scanf_int(input: &[u8], pos: &mut usize) -> i32 {
    // Skip whitespace
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return 0;
    }
    let mut sign: i32 = 1;
    if input[*pos] == b'+' {
        *pos += 1;
    } else if input[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }
    let start = *pos;
    let mut value: i64 = 0;
    while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((input[*pos] - b'0') as i64);
        *pos += 1;
    }
    if *pos == start {
        return 0;
    }
    (value as i32).wrapping_mul(sign)
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();
    let mut pos = 0usize;
    let x = scanf_int(&buf, &mut pos);
    driver(x);
}
