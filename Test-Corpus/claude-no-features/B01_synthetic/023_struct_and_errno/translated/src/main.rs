use std::io::{self, Read, Write};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &House) {
    // Match C's printf %.1f formatting; format!("{:.1}", x) in Rust uses round-half-to-even
    // which matches the typical IEEE 754 default rounding used by glibc's printf.
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = write!(
        h,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(the_house: &mut House, extra_bedrooms: i32) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

/// Reproduce strtol(str, &endp, 10) followed by the original C check:
///   endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX
/// Operates on bytes up to (but not including) a null terminator (matching C string).
fn parse_val(buf: &[u8]) -> Option<i32> {
    // Find C string length (up to null terminator if present).
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let s = &buf[..len];

    let mut i: usize = 0;
    // Skip leading whitespace (matches C isspace in the "C" locale).
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
            _ => break,
        }
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digits.
    let digits_start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            match value.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => value = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits were converted - per C, endp is set back to nptr (the original
        // string start). The check `endp != str` fails, so parse_val returns false.
        return None;
    }

    if overflow {
        // strtol would set errno = ERANGE; the C check would fail.
        return None;
    }

    let result: i64 = if negative {
        match 0i64.checked_sub(value) {
            Some(r) => r,
            None => return None,
        }
    } else {
        value
    };

    if result < i32::MIN as i64 || result > i32::MAX as i64 {
        return None;
    }

    Some(result as i32)
}

/// Mimic fgets(in, 100, stdin): read up to 99 bytes, stopping at (and including)
/// the first '\n', or at EOF. Returns the bytes read (without a trailing null).
fn fgets_99() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < 99 {
        match handle.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn main() {
    // C: char in[100] = "";  -- effectively an empty C string.
    let mut buf = vec![0u8; 100];
    let read = fgets_99();
    // Copy what was read into the start of buf (mirrors fgets writing into in[]).
    // If nothing was read, buf remains all zeros (the empty string), matching C.
    let n = read.len().min(99);
    buf[..n].copy_from_slice(&read[..n]);
    // fgets writes a null terminator after the bytes it read; buf already has zeros.
    if n < buf.len() {
        buf[n] = 0;
    }

    if let Some(x) = parse_val(&buf) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        let stdout = io::stdout();
        let mut h = stdout.lock();
        let _ = write!(h, "An error occurred\n");
    }
}
