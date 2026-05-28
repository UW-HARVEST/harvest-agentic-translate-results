// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output.

use std::ffi::{c_char, c_double, c_int, c_long, c_void};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    fn pow(x: c_double, y: c_double) -> c_double;
}

/// Mimic C's `(int)value` cast on x86_64 (cvttsd2si):
/// - NaN, +Inf, -Inf, and any value outside the i32 range yield INT_MIN.
/// - Otherwise truncate toward zero.
#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: f64) -> c_int {
    if value.is_nan() {
        return c_int::MIN;
    }
    // i32::MIN and i32::MAX are exactly representable as f64.
    let min_f = c_int::MIN as f64; // -2147483648.0
    // c_int::MAX = 2147483647; as f64 is exact, +1.0 = 2147483648.0 exact.
    let upper_excl = (c_int::MAX as f64) + 1.0;
    if value >= min_f && value < upper_excl {
        value as c_int
    } else {
        c_int::MIN
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn find_value_in_buffer(buffer: *const c_char, size: usize, search_val: c_int) -> c_int {
    // Replicate `char target = (char)search_val;` (signed char on x86_64),
    // then sign-extend to int when passing to memchr.
    let target = search_val as i8;
    let result = unsafe { memchr(buffer as *const c_void, target as c_int, size) };
    if !result.is_null() {
        let diff = (result as isize) - (buffer as isize);
        return diff as c_int;
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    // !!var1
    if var1 != 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    // for (int i=0; i<size; i++) buffer[i] = (char)((seed + i*7) % 256);
    for i in 0..size {
        // Use wrapping arithmetic to mimic C int overflow behavior on the operands.
        let val = seed.wrapping_add(i.wrapping_mul(7)) % 256;
        unsafe {
            *buffer.offset(i as isize) = val as c_char;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
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
    let mut buffer: [c_char; 256] = [0; 256];

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
    }

    let negation_test = param1;
    let negation_result = process_negation(negation_test);
    unsafe {
        printf(
            b"Original value: %d\n\0".as_ptr() as *const c_char,
            negation_test,
        );
        printf(
            b"After !!negation: %d\n\0".as_ptr() as *const c_char,
            negation_result,
        );
    }
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2 = process_negation(param2);
    let neg_p3 = process_negation(param3);
    let neg_p4 = process_negation(param4);
    unsafe {
        printf(
            b"Double negation results: %d, %d, %d\n\0".as_ptr() as *const c_char,
            neg_p2,
            neg_p3,
            neg_p4,
        );
    }
    result = result
        .wrapping_add(neg_p2)
        .wrapping_add(neg_p3)
        .wrapping_add(neg_p4);

    unsafe {
        printf(b"\n--- Double to Int Conversion Test ---\n\0".as_ptr() as *const c_char);
    }

    let large_double = calculate_with_doubles(param1, param2, param3);
    unsafe {
        printf(
            b"Calculated double value: %e\n\0".as_ptr() as *const c_char,
            large_double,
        );
    }

    let converted_int = convert_double_to_int(large_double);
    unsafe {
        printf(
            b"Converted to int (may be UB): %d\n\0".as_ptr() as *const c_char,
            converted_int,
        );
    }

    let negative_large = -1.0_f64 * unsafe { pow(2.0, 40.0) };
    unsafe {
        printf(
            b"Very large negative double: %e\n\0".as_ptr() as *const c_char,
            negative_large,
        );
    }
    let converted_neg = convert_double_to_int(negative_large);
    unsafe {
        printf(
            b"Converted to int (UB likely): %d\n\0".as_ptr() as *const c_char,
            converted_neg,
        );
    }

    result = result
        .wrapping_add(converted_int % 1000)
        .wrapping_add(converted_neg % 1000);

    unsafe {
        printf(b"\n--- Memchr Search Test ---\n\0".as_ptr() as *const c_char);
    }

    create_numeric_buffer(buffer.as_mut_ptr(), 256, param1);

    let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
    let num_searches = search_values.len();

    unsafe {
        printf(b"Searching buffer for values...\n\0".as_ptr() as *const c_char);
    }
    for i in 0..num_searches {
        let pos = find_value_in_buffer(buffer.as_ptr(), 256, search_values[i]);
        if pos >= 0 {
            unsafe {
                printf(
                    b"Found value %d at position %d\n\0".as_ptr() as *const c_char,
                    search_values[i],
                    pos,
                );
            }
            result = result.wrapping_add(pos);
        } else {
            unsafe {
                printf(
                    b"Value %d not found\n\0".as_ptr() as *const c_char,
                    search_values[i],
                );
            }
        }
    }

    let direct_search =
        unsafe { memchr(buffer.as_ptr() as *const c_void, 100, 256) } as *mut c_char;
    if !direct_search.is_null() {
        let offset = (direct_search as isize) - (buffer.as_ptr() as isize);
        unsafe {
            printf(
                b"Direct memchr found byte 100 at offset: %ld\n\0".as_ptr() as *const c_char,
                offset as c_long,
            );
        }
        result = result.wrapping_add(offset as c_int);
    }

    unsafe {
        printf(b"\n--- Combined Feature Test ---\n\0".as_ptr() as *const c_char);
    }
    for i in 0..10_i32 {
        let search_byte = param1.wrapping_add(i.wrapping_mul(param2)) % 256;
        let found = unsafe { memchr(buffer.as_ptr() as *const c_void, search_byte, 256) };
        let found_flag: c_int = if !found.is_null() { 1 } else { 0 };
        unsafe {
            printf(
                b"Search %d: byte=%d, found=%d\n\0".as_ptr() as *const c_char,
                i,
                search_byte,
                found_flag,
            );
        }
        result = result.wrapping_add(found_flag);
    }

    let infinity_val = f64::INFINITY;
    let nan_val = f64::NAN;

    unsafe {
        printf(b"\n--- Special Double Values ---\n\0".as_ptr() as *const c_char);
        printf(b"Converting INFINITY to int: \0".as_ptr() as *const c_char);
    }
    let inf_as_int = convert_double_to_int(infinity_val);
    unsafe {
        printf(
            b"%d (undefined behavior)\n\0".as_ptr() as *const c_char,
            inf_as_int,
        );
        printf(b"Converting NAN to int: \0".as_ptr() as *const c_char);
    }
    let nan_as_int = convert_double_to_int(nan_val);
    unsafe {
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
