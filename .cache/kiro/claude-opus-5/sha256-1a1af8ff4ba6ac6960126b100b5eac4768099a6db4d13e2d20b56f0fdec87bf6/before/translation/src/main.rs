// Rust translation of c_src/src/main.c
//
// Behavior is intentionally identical to the original C program, including any
// quirks (e.g. the unchecked `fgets` return value, trailing garbage accepted by
// `strtol`, and the shared mutation of the house across both `run` calls).

use std::io::{self, BufRead, Read, Write};

/// Mirrors `house_t`.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C's `+=` on int wraps in practice on the target platforms.
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn print_house<W: Write>(out: &mut W, house: &House) {
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run<W: Write>(out: &mut W, the_house: &mut House, extra_bedrooms: i32) {
    print_house(out, the_house);
    add_floor(the_house);
    print_house(out, the_house);
    the_house.bathrooms += 1.0;
    print_house(out, the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(out, the_house);
}

fn is_c_space(b: u8) -> bool {
    // C locale isspace(): space, \t, \n, \v, \f, \r
    b == b' ' || (0x09..=0x0d).contains(&b)
}

/// Emulates `strtol(str, &endp, 10)` for a NUL-terminated byte string.
///
/// Returns `(value, endptr_offset, erange)`. When no conversion is performed,
/// `endptr_offset` is 0 (i.e. `endp == str`), matching the C library.
fn strtol_base10(s: &[u8]) -> (i64, usize, bool) {
    let mut i = 0usize;

    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let limit: u128 = if negative {
        (i64::MAX as u128) + 1
    } else {
        i64::MAX as u128
    };
    let mut magnitude: u128 = 0;
    let mut overflow = false;

    while i < s.len() && s[i].is_ascii_digit() {
        if !overflow {
            magnitude = magnitude * 10 + u128::from(s[i] - b'0');
            if magnitude > limit {
                overflow = true;
                magnitude = limit;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: strtol leaves *endptr == str and returns 0.
        return (0, 0, false);
    }

    if overflow {
        let clamped = if negative { i64::MIN } else { i64::MAX };
        return (clamped, i, true); // errno == ERANGE
    }

    let value = if negative {
        (magnitude as u64).wrapping_neg() as i64
    } else {
        magnitude as i64
    };
    (value, i, false)
}

/// Mirrors `parse_val`.
fn parse_val(s: &[u8], val: &mut i32) -> bool {
    // errno = 0;
    let (tmp, endp_offset, erange) = strtol_base10(s);
    let errno_is_zero = !erange;

    if endp_offset != 0
        && errno_is_zero
        && tmp >= i32::MIN as i64
        && tmp <= i32::MAX as i64
    {
        *val = tmp as i32;
        true
    } else {
        false
    }
}

/// Emulates `fgets(in, 100, stdin)` followed by treating `in` as a C string.
///
/// `in` is a 100-byte buffer pre-initialized to all zeros, so at most 99 bytes
/// are read and the effective string stops at the first NUL byte.
fn read_line_fgets() -> Vec<u8> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf: Vec<u8> = Vec::new();
    // fgets stops after a newline (which it keeps), at EOF, or after n-1 bytes.
    let _ = handle.by_ref().take(99).read_until(b'\n', &mut buf);

    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(nul);
    buf
}

fn main() {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let input = read_line_fgets();

    let mut x: i32 = 0;
    if parse_val(&input, &mut x) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut out, &mut the_house, x);
        run(&mut out, &mut the_house, x);
    } else {
        let _ = write!(out, "An error occurred\n");
    }

    let _ = out.flush();
}
