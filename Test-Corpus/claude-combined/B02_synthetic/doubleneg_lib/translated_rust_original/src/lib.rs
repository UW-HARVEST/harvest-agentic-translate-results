// Rust translation of the C library in c_src/.
// Produces byte-identical output by forwarding formatted output to C's
// printf and using C's pow/memchr for arithmetic and search semantics.

use std::ffi::{c_char, c_double, c_int, c_long, c_void};

#[link(name = "m")]
extern "C" {
    fn pow(x: c_double, y: c_double) -> c_double;
}

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

// Mimic x86_64 cvttsd2si behavior used by the C `(int)double` cast.
// If the truncated value does not fit in i32, or value is NaN,
// returns i32::MIN (the "indefinite integer value", 0x80000000).
fn cvttsd2si(value: f64) -> i32 {
    if value.is_nan() {
        return i32::MIN;
    }
    let truncated = value.trunc();
    if truncated >= 2147483648.0 || truncated <= -2147483649.0 {
        return i32::MIN;
    }
    value as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: c_double) -> c_int {
    cvttsd2si(value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    // (char)search_val — on Linux x86_64 char is signed; truncating cast.
    let target = search_val as i8;
    let result = unsafe { memchr(buffer as *const c_void, target as c_int, size) };
    if !result.is_null() {
        let offset = (result as isize) - (buffer as isize);
        return offset as c_int;
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    if var1 != 0 {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_numeric_buffer(
    buffer: *mut c_char,
    size: c_int,
    seed: c_int,
) {
    let mut i: c_int = 0;
    while i < size {
        // (char)((seed + i * 7) % 256) with two's-complement wraparound.
        let val = seed.wrapping_add(i.wrapping_mul(7));
        let byte = val as i8;
        unsafe { *buffer.offset(i as isize) = byte };
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    let mut result: f64 = 0.0;

    if b != 0 {
        result = (a as f64) / (b as f64);
    }

    result *= unsafe { pow(10.0, (c % 10) as f64) };

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: i32 = 0;
    let mut buffer: [c_char; 256] = [0; 256];

    unsafe {
        printf(c"=== Starting foo() execution ===\n".as_ptr());
        printf(
            c"Parameters: %d, %d, %d, %d\n".as_ptr(),
            param1,
            param2,
            param3,
            param4,
        );

        printf(c"\n--- Integer Negation Test ---\n".as_ptr());
    }
    let negation_test = param1;
    let negation_result: c_int = if negation_test != 0 { 1 } else { 0 };
    unsafe {
        printf(c"Original value: %d\n".as_ptr(), negation_test);
        printf(c"After !!negation: %d\n".as_ptr(), negation_result);
    }
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2: c_int = if param2 != 0 { 1 } else { 0 };
    let neg_p3: c_int = if param3 != 0 { 1 } else { 0 };
    let neg_p4: c_int = if param4 != 0 { 1 } else { 0 };
    unsafe {
        printf(
            c"Double negation results: %d, %d, %d\n".as_ptr(),
            neg_p2,
            neg_p3,
            neg_p4,
        );
    }
    result = result.wrapping_add(neg_p2).wrapping_add(neg_p3).wrapping_add(neg_p4);

    unsafe {
        printf(c"\n--- Double to Int Conversion Test ---\n".as_ptr());
    }

    let large_double = calculate_with_doubles(param1, param2, param3);
    unsafe {
        printf(c"Calculated double value: %e\n".as_ptr(), large_double);
    }

    let converted_int = convert_double_to_int(large_double);
    unsafe {
        printf(c"Converted to int (may be UB): %d\n".as_ptr(), converted_int);
    }

    let negative_large = -1.0_f64 * unsafe { pow(2.0, 40.0) };
    unsafe {
        printf(c"Very large negative double: %e\n".as_ptr(), negative_large);
    }
    let converted_neg = convert_double_to_int(negative_large);
    unsafe {
        printf(c"Converted to int (UB likely): %d\n".as_ptr(), converted_neg);
    }

    result = result
        .wrapping_add(converted_int.wrapping_rem(1000))
        .wrapping_add(converted_neg.wrapping_rem(1000));

    unsafe {
        printf(c"\n--- Memchr Search Test ---\n".as_ptr());
    }

    unsafe {
        create_numeric_buffer(buffer.as_mut_ptr(), 256, param1);
    }

    let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
    let num_searches = search_values.len() as c_int;

    unsafe {
        printf(c"Searching buffer for values...\n".as_ptr());
    }
    let mut i: c_int = 0;
    while i < num_searches {
        let pos = unsafe {
            find_value_in_buffer(buffer.as_ptr(), 256, search_values[i as usize])
        };
        if pos >= 0 {
            unsafe {
                printf(
                    c"Found value %d at position %d\n".as_ptr(),
                    search_values[i as usize],
                    pos,
                );
            }
            result = result.wrapping_add(pos);
        } else {
            unsafe {
                printf(c"Value %d not found\n".as_ptr(), search_values[i as usize]);
            }
        }
        i += 1;
    }

    let direct_search = unsafe {
        memchr(buffer.as_ptr() as *const c_void, 100, 256) as *mut c_char
    };
    if !direct_search.is_null() {
        let offset = (direct_search as isize) - (buffer.as_ptr() as isize);
        unsafe {
            printf(
                c"Direct memchr found byte 100 at offset: %ld\n".as_ptr(),
                offset as c_long,
            );
        }
        result = result.wrapping_add(offset as c_int);
    }

    unsafe {
        printf(c"\n--- Combined Feature Test ---\n".as_ptr());
    }
    let mut i: c_int = 0;
    while i < 10 {
        let search_byte = param1.wrapping_add(i.wrapping_mul(param2)) % 256;
        let found = unsafe {
            memchr(buffer.as_ptr() as *const c_void, search_byte, 256)
        };
        let found_flag: c_int = if !found.is_null() { 1 } else { 0 };
        unsafe {
            printf(
                c"Search %d: byte=%d, found=%d\n".as_ptr(),
                i,
                search_byte,
                found_flag,
            );
        }
        result = result.wrapping_add(found_flag);
        i += 1;
    }

    let infinity_val: f64 = f64::INFINITY;
    let nan_val: f64 = f64::NAN;

    unsafe {
        printf(c"\n--- Special Double Values ---\n".as_ptr());
        printf(c"Converting INFINITY to int: ".as_ptr());
    }
    let inf_as_int = convert_double_to_int(infinity_val);
    unsafe {
        printf(c"%d (undefined behavior)\n".as_ptr(), inf_as_int);
    }

    unsafe {
        printf(c"Converting NAN to int: ".as_ptr());
    }
    let nan_as_int = convert_double_to_int(nan_val);
    unsafe {
        printf(c"%d (undefined behavior)\n".as_ptr(), nan_as_int);
    }

    unsafe {
        printf(c"\n=== Final Result ===\n".as_ptr());
        printf(c"Accumulated result: %d\n".as_ptr(), result);
    }

    result
}
