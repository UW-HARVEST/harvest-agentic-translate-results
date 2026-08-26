// Rust translation of c_src/src/main.c
//
// Behaviour is preserved byte-for-byte, including the C library semantics of
// fgets() and strtol() that the original program relies on.
//
// This module holds the actual translation.  It is shared verbatim by
// `src/main.rs` (the `driver` executable) and by `src/lib.rs` (the cdylib that
// exports the same C ABI symbols as the C translation unit: `run` and `main`).

use std::io::{self, Read, Write};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct House {
    pub floors: i32,
    pub bedrooms: i32,
    pub bathrooms: f64,
}

fn add_floor(house: &mut House) {
    // house->floors++
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // house->bedrooms += extra_bedrooms
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn print_house<W: Write>(out: &mut W, house: &House) {
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        house.floors,
        house.bedrooms,
        format_f64_1(house.bathrooms)
    );
}

pub fn run<W: Write>(out: &mut W, the_house: &mut House, extra_bedrooms: i32) {
    print_house(out, the_house);
    add_floor(the_house);
    print_house(out, the_house);
    the_house.bathrooms += 1.0;
    print_house(out, the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(out, the_house);
}

/// Formats a finite double the way C's `printf("%.1f", v)` would.
fn format_f64_1(v: f64) -> String {
    if v.is_nan() {
        // glibc prints "nan" / "-nan"
        return if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if v.is_infinite() {
        return if v < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    // Rust's `{:.1}` formats the *exact* decimal expansion of the double and
    // rounds half-to-even, which is what glibc's `%.1f` does as well, so the
    // fallback below is already byte-identical to glibc.  The fast path is only
    // a shortcut for values that are exact at one decimal place: `round(v*10)`
    // can be at most half an ulp away from `v*10`, i.e. ~1e-16 relative, while a
    // rounding boundary is 0.05 away, so it can never pick a different digit.
    let scaled = v * 10.0;
    if scaled == scaled.trunc() && scaled.abs() < 9.007_199_254_740_992e15 {
        // Exact one-decimal value: no rounding needed at all.
        let neg = scaled < 0.0 || (scaled == 0.0 && v.is_sign_negative());
        let units = scaled.abs() as u64;
        let s = format!("{}.{}", units / 10, units % 10);
        return if neg { format!("-{}", s) } else { s };
    }
    format!("{:.1}", v)
}

/// Emulates `strtol(str, &endp, 10)` for the subset needed here.
/// Returns (value, number_of_bytes_consumed, erange).
fn strtol10(s: &[u8]) -> (i64, usize, bool) {
    let mut i = 0usize;
    // Skip leading whitespace (isspace in the "C" locale).
    while i < s.len()
        && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let digits_start = i;
    let mut acc: i128 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i128;
        if !overflow {
            acc = acc * 10 + d;
            if acc > (i64::MAX as i128) + 1 {
                overflow = true;
            }
        }
        i += 1;
    }
    if i == digits_start {
        // No conversion performed: endptr is set to the original string.
        return (0, 0, false);
    }
    let signed: i128 = if negative { -acc } else { acc };
    if overflow || signed > i64::MAX as i128 || signed < i64::MIN as i128 {
        let clamped = if negative { i64::MIN } else { i64::MAX };
        return (clamped, i, true);
    }
    (signed as i64, i, false)
}

fn parse_val(str_bytes: &[u8], val: &mut i32) -> bool {
    // errno = 0; strtol(str, &endp, 10);
    let (tmp, consumed, erange) = strtol10(str_bytes);
    let endp_moved = consumed != 0; // endp != str
    if endp_moved && !erange && tmp >= i32::MIN as i64 && tmp <= i32::MAX as i64 {
        *val = tmp as i32;
        true
    } else {
        false
    }
}

/// Emulates `fgets(in, sizeof(in), stdin)` with a 100-byte buffer that was
/// initialised to all zero bytes.  Returns the buffer contents.
fn fgets(buf: &mut [u8]) {
    let cap = buf.len();
    if cap == 0 {
        return;
    }
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    let mut n = 0usize;
    while n + 1 < cap {
        match handle.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf[n] = byte[0];
                n += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    if n > 0 {
        buf[n] = 0;
    }
}

/// The body of the C `main()`.
pub fn program_main() {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut in_buf = [0u8; 100];
    fgets(&mut in_buf[..]);

    // The buffer is used as a NUL-terminated C string.
    let end = in_buf.iter().position(|&b| b == 0).unwrap_or(in_buf.len());
    let c_str = &in_buf[..end];

    let mut x: i32 = 0;
    if parse_val(c_str, &mut x) {
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
