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
    let mut out = stdout.lock();
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    out.write_all(s.as_bytes()).unwrap();
}

#[no_mangle]
pub extern "C" fn driver(floors: i32) {
    // {0} initialization zeroes the entire struct including padding.
    let size = std::mem::size_of::<House>();
    let mut raw_struct = vec![0u8; size];

    // Build the house with zero-initialized memory, then assign fields.
    // We do this by directly writing field bytes to a zeroed buffer to match
    // the C struct's `{0}` zero-initialization semantics (including padding).
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };

    // Use unsafe transmute via copy into a zeroed buffer to ensure padding bytes
    // are exactly the zero bytes from the original `{0}` initialization.
    // First, zero the buffer (already zeroed). Then write each field into its offset.
    let floors_bytes = house.floors.to_ne_bytes();
    let bedrooms_bytes = house.bedrooms.to_ne_bytes();
    let bathrooms_bytes = house.bathrooms.to_ne_bytes();

    // Compute offsets matching the C ABI: floors at 0, bedrooms at 4, bathrooms at 8.
    raw_struct[0..4].copy_from_slice(&floors_bytes);
    raw_struct[4..8].copy_from_slice(&bedrooms_bytes);
    raw_struct[8..16].copy_from_slice(&bathrooms_bytes);

    // Copy to "raw" buffer like memcpy in C.
    let mut raw = vec![0u8; size];
    raw.copy_from_slice(&raw_struct);

    print_hex(&raw);
}

fn read_scanf_int() -> i32 {
    // Mimic C's scanf("%d", &x): skip leading whitespace (including newlines),
    // then read an optional sign and digits.
    let mut buf = [0u8; 1];
    let mut stdin = io::stdin();

    let c: Option<u8>;

    // Skip whitespace
    loop {
        match stdin.read(&mut buf) {
            Ok(0) => {
                c = None;
                break;
            }
            Ok(_) => {
                let ch = buf[0];
                if (ch as char).is_ascii_whitespace() {
                    continue;
                } else {
                    c = Some(ch);
                    break;
                }
            }
            Err(_) => {
                c = None;
                break;
            }
        }
    }

    if c.is_none() {
        return 0;
    }

    let mut digits = Vec::new();
    let mut ch = c.unwrap();

    if ch == b'+' || ch == b'-' {
        digits.push(ch);
        match stdin.read(&mut buf) {
            Ok(0) => return 0,
            Ok(_) => ch = buf[0],
            Err(_) => return 0,
        }
    }

    if !ch.is_ascii_digit() {
        return 0;
    }

    digits.push(ch);

    loop {
        match stdin.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                let ch = buf[0];
                if ch.is_ascii_digit() {
                    digits.push(ch);
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let s = std::str::from_utf8(&digits).unwrap_or("0");
    s.parse::<i32>().unwrap_or(0)
}

/// Exposed under the C symbol `main` so the Rust .so exports the same
/// `main` symbol as the C .so. Returns 0 on success, matching the C
/// program's `return 0;`.
///
/// Gated off `#[cfg(test)]` so it does not collide with the test
/// harness's auto-generated `main` entry point.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    bin_main();
    0
}

/// Pure-Rust callable entry point used by the binary target — keeps
/// the bin from clashing with the `#[no_mangle] main` symbol above.
pub fn bin_main() {
    let x = read_scanf_int();
    driver(x);
}
