// Rust translation of c_src/src/lib.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::{c_char, c_double, c_int, c_long};
use std::io::Write;

// ---------------------------------------------------------------------------
// printf("%e", ...) emulation
// ---------------------------------------------------------------------------

/// Formats `v` exactly the way glibc's `printf("%e", v)` does (default
/// precision of 6): one leading digit, a decimal point, six fraction digits,
/// `e`, an explicit exponent sign, and at least two exponent digits.
///
/// Rust's `{:.6e}` performs the same correctly-rounded (ties-to-even) decimal
/// conversion as glibc, so only the exponent field needs reshaping.
fn fmt_e(v: f64) -> String {
    if v.is_nan() {
        // glibc honours the sign bit of a NaN for %e.
        return if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if v.is_infinite() {
        return if v.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }

    let s = format!("{:.6e}", v);
    // `s` looks like "-1.099512e12" / "0.000000e0" / "1.000000e-9".
    let (mantissa, exp) = s.split_once('e').expect("{:e} always emits an exponent");
    let (sign, digits) = match exp.strip_prefix('-') {
        Some(rest) => ('-', rest),
        None => ('+', exp),
    };
    format!("{mantissa}e{sign}{digits:0>2}")
}

// ---------------------------------------------------------------------------
// C semantics helpers
// ---------------------------------------------------------------------------

/// Reproduces `(int)value` for a `double` on x86-64 (the `cvttsd2si`
/// instruction): truncation toward zero, and the "integer indefinite" value
/// `INT_MIN` whenever the result is not representable (including NaN and the
/// infinities). The original C relies on this undefined behaviour, so the
/// platform result is reproduced rather than corrected.
fn c_double_to_int(value: f64) -> i32 {
    if value.is_nan() {
        return i32::MIN;
    }
    let truncated = value.trunc();
    if truncated >= -2147483648.0 && truncated <= 2147483647.0 {
        truncated as i32
    } else {
        i32::MIN
    }
}

/// `memchr`: the needle is compared as an `unsigned char`.
fn c_memchr(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

// ---------------------------------------------------------------------------
// Internal (safe) implementations
// ---------------------------------------------------------------------------

fn find_value_in_buffer_impl(buffer: &[u8], search_val: i32) -> i32 {
    // `char target = (char)search_val;` then `memchr` re-reads it as an
    // unsigned char, so only the low 8 bits matter.
    let target = search_val as u8;
    match c_memchr(buffer, target) {
        Some(index) => index as i32,
        None => -1,
    }
}

fn create_numeric_buffer_impl(buffer: &mut [u8], seed: i32) {
    for (i, slot) in buffer.iter_mut().enumerate() {
        // `(seed + i * 7) % 256` with C's truncating remainder, then narrowed
        // to `char` (low 8 bits). Wrapping arithmetic mirrors what the C does
        // in practice when `seed + i * 7` overflows `int`.
        let value = seed.wrapping_add((i as i32).wrapping_mul(7)).wrapping_rem(256);
        *slot = value as u8;
    }
}

fn calculate_with_doubles_impl(a: i32, b: i32, c: i32) -> f64 {
    let mut result = 0.0f64;

    if b != 0 {
        result = f64::from(a) / f64::from(b);
    }

    result *= 10.0f64.powf(f64::from(c.wrapping_rem(10)));

    result
}

// ---------------------------------------------------------------------------
// Exported C API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: c_double) -> c_int {
    c_double_to_int(value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    let bytes = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), size) };
    find_value_in_buffer_impl(bytes, search_val)
}

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    // `!!var1`
    c_int::from(var1 != 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    if size <= 0 {
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts_mut(buffer.cast::<u8>(), size as usize) };
    create_numeric_buffer_impl(bytes, seed);
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    calculate_with_doubles_impl(a, b, c)
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let out = std::io::stdout();
    let mut out = out.lock();
    let mut result: i32 = 0;
    let mut buffer = [0u8; 256];

    let _ = write!(out, "=== Starting foo() execution ===\n");
    let _ = write!(
        out,
        "Parameters: {}, {}, {}, {}\n",
        param1, param2, param3, param4
    );

    let _ = write!(out, "\n--- Integer Negation Test ---\n");
    let negation_test = param1;
    let negation_result = i32::from(negation_test != 0);
    let _ = write!(out, "Original value: {}\n", negation_test);
    let _ = write!(out, "After !!negation: {}\n", negation_result);
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2 = i32::from(param2 != 0);
    let neg_p3 = i32::from(param3 != 0);
    let neg_p4 = i32::from(param4 != 0);
    let _ = write!(
        out,
        "Double negation results: {}, {}, {}\n",
        neg_p2, neg_p3, neg_p4
    );
    result = result
        .wrapping_add(neg_p2)
        .wrapping_add(neg_p3)
        .wrapping_add(neg_p4);

    let _ = write!(out, "\n--- Double to Int Conversion Test ---\n");

    let large_double = calculate_with_doubles_impl(param1, param2, param3);
    let _ = write!(out, "Calculated double value: {}\n", fmt_e(large_double));

    let converted_int = c_double_to_int(large_double);
    let _ = write!(out, "Converted to int (may be UB): {}\n", converted_int);

    let negative_large = -1.0f64 * 2.0f64.powf(40.0);
    let _ = write!(out, "Very large negative double: {}\n", fmt_e(negative_large));
    let converted_neg = c_double_to_int(negative_large);
    let _ = write!(out, "Converted to int (UB likely): {}\n", converted_neg);

    result = result
        .wrapping_add(converted_int.wrapping_rem(1000))
        .wrapping_add(converted_neg.wrapping_rem(1000));

    let _ = write!(out, "\n--- Memchr Search Test ---\n");

    create_numeric_buffer_impl(&mut buffer, param1);

    let search_values: [i32; 4] = [
        param2.wrapping_rem(256),
        param3.wrapping_rem(256),
        param4.wrapping_rem(256),
        42,
    ];

    let _ = write!(out, "Searching buffer for values...\n");
    for &search_value in search_values.iter() {
        let pos = find_value_in_buffer_impl(&buffer, search_value);
        if pos >= 0 {
            let _ = write!(out, "Found value {} at position {}\n", search_value, pos);
            result = result.wrapping_add(pos);
        } else {
            let _ = write!(out, "Value {} not found\n", search_value);
        }
    }

    if let Some(offset) = c_memchr(&buffer, 100) {
        let _ = write!(
            out,
            "Direct memchr found byte 100 at offset: {}\n",
            offset as c_long
        );
        result = result.wrapping_add(offset as i32);
    }

    let _ = write!(out, "\n--- Combined Feature Test ---\n");
    for i in 0..10i32 {
        let search_byte = param1.wrapping_add(i.wrapping_mul(param2)).wrapping_rem(256);
        let found = c_memchr(&buffer, search_byte as u8);
        let found_flag = i32::from(found.is_some()); // Double negation on pointer
        let _ = write!(
            out,
            "Search {}: byte={}, found={}\n",
            i, search_byte, found_flag
        );
        result = result.wrapping_add(found_flag);
    }

    let infinity_val = f64::INFINITY;
    let nan_val = f64::NAN;

    let _ = write!(out, "\n--- Special Double Values ---\n");
    let _ = write!(out, "Converting INFINITY to int: ");
    let inf_as_int = c_double_to_int(infinity_val);
    let _ = write!(out, "{} (undefined behavior)\n", inf_as_int);

    let _ = write!(out, "Converting NAN to int: ");
    let nan_as_int = c_double_to_int(nan_val);
    let _ = write!(out, "{} (undefined behavior)\n", nan_as_int);

    let _ = write!(out, "\n=== Final Result ===\n");
    let _ = write!(out, "Accumulated result: {}\n", result);

    let _ = out.flush();

    result
}
