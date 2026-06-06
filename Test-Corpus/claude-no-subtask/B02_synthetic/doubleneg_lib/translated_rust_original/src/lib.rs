// Translation of c_src/src/lib.c to Rust.
//
// This crate is a cdylib exposing the C-compatible function `doubleneg`.
// To preserve byte-identical output, all formatted I/O is delegated to the
// C library's printf, and we emulate the x86-64 cvttsd2si semantics for
// double->int conversion (returning INT_MIN for NaN, infinity, or values
// outside the i32 range).

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_double, c_int, c_long, c_void};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    fn pow(x: c_double, y: c_double) -> c_double;
}

/// Match the x86-64 `cvttsd2si` semantics used by gcc/clang for `(int)d`.
/// NaN, +/-Inf, or any value outside the [INT_MIN, INT_MAX] range yields
/// the "indefinite" result of 0x80000000 (i.e. i32::MIN).
fn convert_double_to_int(value: f64) -> c_int {
    if value.is_nan() || value >= 2147483648.0 || value < -2147483648.0 {
        return c_int::MIN;
    }
    value as c_int
}

fn find_value_in_buffer(buffer: *const c_char, size: usize, search_val: c_int) -> c_int {
    // Replicate `char target = (char)search_val;` then call memchr.
    let target: i8 = search_val as i8;
    unsafe {
        let result = memchr(buffer as *const c_void, target as c_int, size);
        if !result.is_null() {
            return (result as *const c_char).offset_from(buffer) as c_int;
        }
    }
    -1
}

#[allow(dead_code)]
fn process_negation(var1: c_int) -> c_int {
    if var1 != 0 { 1 } else { 0 }
}

fn create_numeric_buffer(buffer: &mut [u8], size: c_int, seed: c_int) {
    let mut i: c_int = 0;
    while i < size {
        // C: buffer[i] = (char)((seed + i * 7) % 256);
        // Use wrapping ops to mirror two's-complement int arithmetic.
        let v = seed.wrapping_add(i.wrapping_mul(7)) % 256;
        buffer[i as usize] = v as i8 as u8;
        i += 1;
    }
}

fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
    let mut result: f64 = 0.0;
    if b != 0 {
        result = (a as f64) / (b as f64);
    }
    let exp = (c % 10) as f64;
    result *= unsafe { pow(10.0, exp) };
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: [u8; 256] = [0u8; 256];

    unsafe {
        printf(b"=== Starting foo() execution ===\n\0".as_ptr() as *const c_char);
        printf(
            b"Parameters: %d, %d, %d, %d\n\0".as_ptr() as *const c_char,
            param1,
            param2,
            param3,
            param4,
        );

        printf(b"\n--- Integer Negation Test ---\n\0".as_ptr() as *const c_char);
        let negation_test: c_int = param1;
        let negation_result: c_int = if negation_test != 0 { 1 } else { 0 };
        printf(
            b"Original value: %d\n\0".as_ptr() as *const c_char,
            negation_test,
        );
        printf(
            b"After !!negation: %d\n\0".as_ptr() as *const c_char,
            negation_result,
        );
        result = result.wrapping_add(negation_result.wrapping_mul(10));

        let neg_p2: c_int = if param2 != 0 { 1 } else { 0 };
        let neg_p3: c_int = if param3 != 0 { 1 } else { 0 };
        let neg_p4: c_int = if param4 != 0 { 1 } else { 0 };
        printf(
            b"Double negation results: %d, %d, %d\n\0".as_ptr() as *const c_char,
            neg_p2,
            neg_p3,
            neg_p4,
        );
        result = result
            .wrapping_add(neg_p2)
            .wrapping_add(neg_p3)
            .wrapping_add(neg_p4);

        printf(b"\n--- Double to Int Conversion Test ---\n\0".as_ptr() as *const c_char);

        let large_double = calculate_with_doubles(param1, param2, param3);
        printf(
            b"Calculated double value: %e\n\0".as_ptr() as *const c_char,
            large_double,
        );

        let converted_int = convert_double_to_int(large_double);
        printf(
            b"Converted to int (may be UB): %d\n\0".as_ptr() as *const c_char,
            converted_int,
        );

        let negative_large: f64 = -1.0_f64 * pow(2.0, 40.0);
        printf(
            b"Very large negative double: %e\n\0".as_ptr() as *const c_char,
            negative_large,
        );
        let converted_neg = convert_double_to_int(negative_large);
        printf(
            b"Converted to int (UB likely): %d\n\0".as_ptr() as *const c_char,
            converted_neg,
        );

        result = result
            .wrapping_add(converted_int % 1000)
            .wrapping_add(converted_neg % 1000);

        printf(b"\n--- Memchr Search Test ---\n\0".as_ptr() as *const c_char);

        create_numeric_buffer(&mut buffer, 256, param1);

        let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
        let num_searches: usize = search_values.len();

        printf(b"Searching buffer for values...\n\0".as_ptr() as *const c_char);
        for i in 0..num_searches {
            let pos = find_value_in_buffer(
                buffer.as_ptr() as *const c_char,
                256,
                search_values[i],
            );
            if pos >= 0 {
                printf(
                    b"Found value %d at position %d\n\0".as_ptr() as *const c_char,
                    search_values[i],
                    pos,
                );
                result = result.wrapping_add(pos);
            } else {
                printf(
                    b"Value %d not found\n\0".as_ptr() as *const c_char,
                    search_values[i],
                );
            }
        }

        let direct_search =
            memchr(buffer.as_ptr() as *const c_void, 100, 256) as *const c_char;
        if !direct_search.is_null() {
            let off: c_long =
                direct_search.offset_from(buffer.as_ptr() as *const c_char) as c_long;
            printf(
                b"Direct memchr found byte 100 at offset: %ld\n\0".as_ptr() as *const c_char,
                off,
            );
            result = result.wrapping_add(off as c_int);
        }

        printf(b"\n--- Combined Feature Test ---\n\0".as_ptr() as *const c_char);
        for i in 0..10i32 {
            let search_byte: c_int =
                param1.wrapping_add(i.wrapping_mul(param2)) % 256;
            let found = memchr(buffer.as_ptr() as *const c_void, search_byte, 256);
            let found_flag: c_int = if !found.is_null() { 1 } else { 0 };
            printf(
                b"Search %d: byte=%d, found=%d\n\0".as_ptr() as *const c_char,
                i,
                search_byte,
                found_flag,
            );
            result = result.wrapping_add(found_flag);
        }

        let infinity_val: f64 = f64::INFINITY;
        let nan_val: f64 = f64::NAN;

        printf(b"\n--- Special Double Values ---\n\0".as_ptr() as *const c_char);
        printf(b"Converting INFINITY to int: \0".as_ptr() as *const c_char);
        let inf_as_int = convert_double_to_int(infinity_val);
        printf(
            b"%d (undefined behavior)\n\0".as_ptr() as *const c_char,
            inf_as_int,
        );

        printf(b"Converting NAN to int: \0".as_ptr() as *const c_char);
        let nan_as_int = convert_double_to_int(nan_val);
        printf(
            b"%d (undefined behavior)\n\0".as_ptr() as *const c_char,
            nan_as_int,
        );

        printf(b"\n=== Final Result ===\n\0".as_ptr() as *const c_char);
        printf(
            b"Accumulated result: %d\n\0".as_ptr() as *const c_char,
            result,
        );
    }

    result
}
