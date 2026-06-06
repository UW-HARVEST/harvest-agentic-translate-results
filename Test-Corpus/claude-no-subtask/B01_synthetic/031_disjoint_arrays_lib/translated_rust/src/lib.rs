// Translation of c_src/src/driver.c to Rust.
// Produces byte-identical output to the C version.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Multiply-add over arrays. out[i] = mul1[i] * mul2[i] + add[i]
/// Mirrors the C function `fma_array`.
fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        // Match C wrapping semantics for int.
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

/// Mirrors the C function `call_fma`.
fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let mut ones = vec![0i32; len];
    let mut zeros = vec![0i32; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, &data, &zeros, len);
    out[len - 1]
}

/// Replicates `sscanf(in, "%d%zn", &value, &nb)`.
///
/// Returns Some((value, bytes_consumed)) on success, or None on matching/input failure.
///
/// The C `%d` conversion:
///   * Skips leading whitespace as defined by `isspace`.
///   * Optionally consumes a sign (`+` or `-`).
///   * Consumes one or more decimal digits.
///   * Stops at the first non-digit character.
///   * Overflow is undefined behavior in C; we replicate the common behavior of
///     wrapping arithmetic on `int` (i32).
///
/// `nb` reports the total number of input bytes consumed up to and including
/// the last digit of the converted integer (i.e. relative to the original
/// string passed to sscanf at this call).
fn sscanf_int(input: &[u8]) -> Option<(i32, usize)> {
    let mut pos = 0usize;

    // Skip leading whitespace (matches C isspace for the standard ASCII set).
    while pos < input.len() && is_c_space(input[pos]) {
        pos += 1;
    }

    if pos >= input.len() {
        // Input failure (no characters to match).
        return None;
    }

    // Optional sign.
    let mut negative = false;
    if input[pos] == b'+' {
        pos += 1;
    } else if input[pos] == b'-' {
        negative = true;
        pos += 1;
    }

    // Must have at least one digit.
    let digit_start = pos;
    let mut value: i32 = 0;
    while pos < input.len() && input[pos].is_ascii_digit() {
        let d = (input[pos] - b'0') as i32;
        // Wrapping mul/add to mimic typical C behavior on overflow.
        value = value.wrapping_mul(10);
        if negative {
            value = value.wrapping_sub(d);
        } else {
            value = value.wrapping_add(d);
        }
        pos += 1;
    }

    if pos == digit_start {
        // Matching failure: no digits.
        return None;
    }

    Some((value, pos))
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Public C entry point: `void driver(const char *in)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    if input.is_null() {
        return;
    }

    // Determine the length of the C string.
    let mut len = 0usize;
    while *input.add(len) != 0 {
        len += 1;
    }
    let bytes = std::slice::from_raw_parts(input as *const u8, len);

    let mut data: [i32; 100] = [0; 100];
    let mut i: usize = 0;
    let mut offset: usize = 0;

    while i < 100 {
        match sscanf_int(&bytes[offset..]) {
            Some((value, consumed)) => {
                data[i] = value;
                offset += consumed;
                i += 1;
            }
            None => break,
        }
    }

    let result = call_fma(&data[..i], i);

    // Use libc's printf to match the exact byte output of the C version,
    // including stdio's buffering semantics.
    let fmt = b"%d\n\0";
    printf(fmt.as_ptr() as *const c_char, result as c_int);
}
