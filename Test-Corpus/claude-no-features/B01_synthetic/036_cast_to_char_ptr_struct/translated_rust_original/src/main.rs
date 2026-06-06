use std::io::{self, Read, Write, BufWriter};

#[repr(C)]
#[derive(Default)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex<W: Write>(out: &mut W, bytes: &[u8]) {
    for b in bytes {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver<W: Write>(out: &mut W, floors: i32) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    // SAFETY: We need a byte view of the C-compatible struct including any
    // padding bytes. The struct is zero-initialized via Default for any
    // padding, and #[repr(C)] guarantees layout matching the C struct.
    let size = std::mem::size_of::<House>();
    let ptr = &house as *const House as *const u8;
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(ptr, size) };
    print_hex(out, bytes);
}

/// Read a single integer in scanf("%d", ...) style: skip leading whitespace,
/// then accept an optional sign followed by decimal digits.
fn scanf_int<R: Read>(input: &mut R) -> i32 {
    let mut byte = [0u8; 1];
    // Skip leading whitespace
    loop {
        match input.read(&mut byte) {
            Ok(0) => return 0,
            Ok(_) => {
                let c = byte[0];
                if !(c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 11 || c == 12) {
                    break;
                }
            }
            Err(_) => return 0,
        }
    }

    let mut sign: i32 = 1;
    let c = byte[0];
    let mut have_byte = true;

    if c == b'+' || c == b'-' {
        if c == b'-' {
            sign = -1;
        }
        have_byte = false;
    }

    let mut value: i32 = 0;
    let mut any_digit = false;

    loop {
        if !have_byte {
            match input.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        have_byte = false;
        let c = byte[0];
        if c.is_ascii_digit() {
            any_digit = true;
            // Use wrapping arithmetic to mimic C's signed overflow behavior
            // (technically UB in C, but consistent on common platforms).
            value = value
                .wrapping_mul(10)
                .wrapping_add((c - b'0') as i32 * sign);
        } else {
            break;
        }
    }

    if !any_digit {
        return 0;
    }
    value
}

fn main() {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let stdin = io::stdin();
    let mut input = stdin.lock();

    let x = scanf_int(&mut input);
    driver(&mut out, x);
}
