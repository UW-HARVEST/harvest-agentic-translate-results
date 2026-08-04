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
    print!(
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

/// Mimic C's fgets: read up to (n - 1) bytes from stdin, stopping at newline (which is
/// included in the result) or EOF. Returns the bytes read (without a trailing NUL).
fn c_fgets(n: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    if n == 0 {
        return buf;
    }
    let limit = n - 1;
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < limit {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
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

/// Mimic C's strtol with base 10, then range-check against i32.
/// Returns Some(val) if at least one digit was parsed (after optional sign and
/// leading whitespace) and the value fits in an i32; otherwise None.
///
/// On overflow (ERANGE), strtol returns LONG_MIN/LONG_MAX with errno set, which
/// our wrapper treats as a parse failure.
fn parse_val(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // Skip leading whitespace (C isspace in the "C" locale).
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => i += 1,
            _ => break,
        }
    }
    let sign_start = i;
    let mut negative = false;
    if i < s.len() {
        match s[i] {
            b'+' => {
                i += 1;
            }
            b'-' => {
                negative = true;
                i += 1;
            }
            _ => {}
        }
    }
    let digit_start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    // C's strtol: if no digits were consumed, endp points to the original
    // string in glibc, so our caller's `endp != str` check fails. The
    // (unused) sign_start kept for clarity / parity with the C original.
    let _ = sign_start;
    if i == digit_start {
        return None;
    }

    let digits = &s[digit_start..i];
    // Build the value as i64 to detect overflow vs i32 range; on long overflow
    // strtol would set errno = ERANGE — emulate that by treating overflow as
    // failure.
    let mut acc: i64 = 0;
    let mut overflow = false;
    for &d in digits {
        let v = (d - b'0') as i64;
        if negative {
            match acc.checked_mul(10).and_then(|a| a.checked_sub(v)) {
                Some(next) => acc = next,
                None => {
                    overflow = true;
                    break;
                }
            }
        } else {
            match acc.checked_mul(10).and_then(|a| a.checked_add(v)) {
                Some(next) => acc = next,
                None => {
                    overflow = true;
                    break;
                }
            }
        }
    }

    if overflow {
        return None;
    }

    if acc < i32::MIN as i64 || acc > i32::MAX as i64 {
        return None;
    }

    Some(acc as i32)
}

fn main() {
    // C: char in[100] = ""; fgets(in, sizeof(in), stdin);
    let buf = c_fgets(100);

    match parse_val(&buf) {
        Some(x) => {
            let mut the_house = House {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            };
            run(&mut the_house, x);
            run(&mut the_house, x);
        }
        None => {
            print!("An error occurred\n");
        }
    }

    // Ensure all output is flushed.
    let _ = io::stdout().flush();
}
