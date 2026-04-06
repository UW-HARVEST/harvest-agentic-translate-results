use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn memchr(s: *const std::ffi::c_void, c: c_int, n: usize) -> *mut std::ffi::c_void;
    fn pow(base: f64, exp: f64) -> f64;
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: f64) -> c_int {
    value as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn find_value_in_buffer(buffer: *const c_char, size: usize, search_val: c_int) -> c_int {
    let target = search_val as c_char;
    let result = unsafe { memchr(buffer as *const std::ffi::c_void, target as c_int, size) };
    if !result.is_null() {
        return (result as isize - buffer as isize) as c_int;
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    if var1 != 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    for i in 0..size {
        unsafe {
            *buffer.offset(i as isize) = ((seed.wrapping_add(i.wrapping_mul(7))) % 256) as c_char;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
    let mut result: f64 = 0.0;
    if b != 0 {
        result = a as f64 / b as f64;
    }
    result *= unsafe { pow(10.0, (c % 10) as f64) };
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        let mut result: c_int = 0;
        let mut buffer = [0i8; 256];

        printf(b"=== Starting foo() execution ===\n\0".as_ptr() as *const c_char);
        printf(
            b"Parameters: %d, %d, %d, %d\n\0".as_ptr() as *const c_char,
            param1, param2, param3, param4,
        );

        printf(b"\n--- Integer Negation Test ---\n\0".as_ptr() as *const c_char);
        let negation_test = param1;
        let negation_result: c_int = if negation_test != 0 { 1 } else { 0 };
        printf(
            b"Original value: %d\n\0".as_ptr() as *const c_char,
            negation_test,
        );
        printf(
            b"After !!negation: %d\n\0".as_ptr() as *const c_char,
            negation_result,
        );
        result += negation_result * 10;

        let neg_p2: c_int = if param2 != 0 { 1 } else { 0 };
        let neg_p3: c_int = if param3 != 0 { 1 } else { 0 };
        let neg_p4: c_int = if param4 != 0 { 1 } else { 0 };
        printf(
            b"Double negation results: %d, %d, %d\n\0".as_ptr() as *const c_char,
            neg_p2, neg_p3, neg_p4,
        );
        result += neg_p2 + neg_p3 + neg_p4;

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

        let negative_large: f64 = -1.0 * pow(2.0, 40.0);
        printf(
            b"Very large negative double: %e\n\0".as_ptr() as *const c_char,
            negative_large,
        );
        let converted_neg = convert_double_to_int(negative_large);
        printf(
            b"Converted to int (UB likely): %d\n\0".as_ptr() as *const c_char,
            converted_neg,
        );

        result += (converted_int % 1000) + (converted_neg % 1000);

        printf(b"\n--- Memchr Search Test ---\n\0".as_ptr() as *const c_char);

        create_numeric_buffer(buffer.as_mut_ptr(), 256, param1);

        let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
        let num_searches = 4;

        printf(b"Searching buffer for values...\n\0".as_ptr() as *const c_char);
        for i in 0..num_searches {
            let pos = find_value_in_buffer(buffer.as_ptr(), 256, search_values[i]);
            if pos >= 0 {
                printf(
                    b"Found value %d at position %d\n\0".as_ptr() as *const c_char,
                    search_values[i], pos,
                );
                result += pos;
            } else {
                printf(
                    b"Value %d not found\n\0".as_ptr() as *const c_char,
                    search_values[i],
                );
            }
        }

        let direct_search = memchr(buffer.as_ptr() as *const std::ffi::c_void, 100, 256);
        if !direct_search.is_null() {
            printf(
                b"Direct memchr found byte 100 at offset: %ld\n\0".as_ptr() as *const c_char,
                (direct_search as isize - buffer.as_ptr() as isize) as std::ffi::c_long,
            );
            result += (direct_search as isize - buffer.as_ptr() as isize) as c_int;
        }

        printf(b"\n--- Combined Feature Test ---\n\0".as_ptr() as *const c_char);
        for i in 0..10 {
            let search_byte = (param1 + i * param2) % 256;
            let found = memchr(buffer.as_ptr() as *const std::ffi::c_void, search_byte, 256);
            let found_flag: c_int = if !found.is_null() { 1 } else { 0 };
            printf(
                b"Search %d: byte=%d, found=%d\n\0".as_ptr() as *const c_char,
                i, search_byte, found_flag,
            );
            result += found_flag;
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

        result
    }
}
