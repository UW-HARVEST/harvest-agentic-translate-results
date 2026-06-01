use std::io::{self, Read, Write};

#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    out.write_all(s.as_bytes()).unwrap();
}

fn driver(floors: i32) {
    // Mimic C struct {0} initialization, then field assignments.
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let size = std::mem::size_of::<House>();
    let p = &house as *const House as *const u8;
    // Safe slice from the struct's memory representation
    let bytes = unsafe { std::slice::from_raw_parts(p, size) };
    print_hex(bytes);
}

/// Mimic C's `scanf("%d", &x)` for a single integer.
/// - Skips leading whitespace
/// - Optional sign + digits
/// - Stops on first non-digit
/// - If no valid int is parsed, returns None (caller should keep x = 0)
fn scan_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < input.len() {
        let c = input[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    let mut negative = false;
    if input[*pos] == b'-' {
        negative = true;
        *pos += 1;
    } else if input[*pos] == b'+' {
        *pos += 1;
    }
    let digits_start = *pos;
    let mut value: i64 = 0;
    while *pos < input.len() {
        let c = input[*pos];
        if c.is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos == digits_start {
        // No digits -> no conversion. Reset pos to start (matchfailure, but pos shouldn't
        // matter since main only calls scanf once).
        *pos = start;
        return None;
    }
    if negative {
        value = -value;
    }
    Some(value as i32)
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).ok();
    let mut pos = 0usize;
    let mut x: i32 = 0;
    if let Some(v) = scan_int(&input, &mut pos) {
        x = v;
    }
    driver(x);
}
