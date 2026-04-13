use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_double;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: [c_char; 256] = [0; 256];

    println!("=== Starting foo() execution ===");
    println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

    println!("\n--- Integer Negation Test ---");
    let negation_test: c_int = param1;
    let negation_result: c_int = if negation_test != 0 { 1 } else { 0 };
    println!("Original value: {}", negation_test);
    println!("After !!negation: {}", negation_result);
    result += negation_result * 10;

    let neg_p2: c_int = if param2 != 0 { 1 } else { 0 };
    let neg_p3: c_int = if param3 != 0 { 1 } else { 0 };
    let neg_p4: c_int = if param4 != 0 { 1 } else { 0 };
    println!("Double negation results: {}, {}, {}", neg_p2, neg_p3, neg_p4);
    result += neg_p2 + neg_p3 + neg_p4;

    println!("\n--- Double to Int Conversion Test ---");

    let large_double: c_double = calculate_with_doubles(param1, param2, param3);
    println!("Calculated double value: {:e}", large_double);

    let converted_int: c_int = convert_double_to_int(large_double);
    println!("Converted to int (may be UB): {}", converted_int);

    let negative_large: c_double = -1.0 * 2.0_f64.powi(40);
    println!("Very large negative double: {:e}", negative_large);
    let converted_neg: c_int = convert_double_to_int(negative_large);
    println!("Converted to int (UB likely): {}", converted_neg);

    result += (converted_int % 1000) + (converted_neg % 1000);

    println!("\n--- Memchr Search Test ---");

    create_numeric_buffer(buffer.as_mut_ptr(), 256, param1);

    let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
    let num_searches: usize = search_values.len();

    println!("Searching buffer for values...");
    for i in 0..num_searches {
        let pos: c_int = find_value_in_buffer(buffer.as_ptr(), 256, search_values[i]);
        if pos >= 0 {
            println!("Found value {} at position {}", search_values[i], pos);
            result += pos;
        } else {
            println!("Value {} not found", search_values[i]);
        }
    }

    let direct_search: *const c_char = unsafe {
        libc::memchr(buffer.as_ptr() as *const c_void, 100, 256) as *const c_char
    };
    if !direct_search.is_null() {
        let offset: isize = unsafe { direct_search.offset_from(buffer.as_ptr()) };
        println!("Direct memchr found byte 100 at offset: {}", offset);
        result += offset as c_int;
    }

    println!("\n--- Combined Feature Test ---");
    for i in 0..10 {
        let search_byte: c_int = (param1 + i * param2) % 256;
        let found: *const c_void = unsafe {
            libc::memchr(buffer.as_ptr() as *const c_void, search_byte, 256)
        };
        let found_flag: c_int = if !found.is_null() { 1 } else { 0 };
        println!("Search {}: byte={}, found={}", i, search_byte, found_flag);
        result += found_flag;
    }

    let infinity_val: c_double = f64::INFINITY;
    let nan_val: c_double = f64::NAN;

    println!("\n--- Special Double Values ---");
    print!("Converting INFINITY to int: ");
    let inf_as_int: c_int = convert_double_to_int(infinity_val);
    println!("{} (undefined behavior)", inf_as_int);

    print!("Converting NAN to int: ");
    let nan_as_int: c_int = convert_double_to_int(nan_val);
    println!("{} (undefined behavior)", nan_as_int);

    println!("\n=== Final Result ===");
    println!("Accumulated result: {}", result);

    result
}

fn convert_double_to_int(value: c_double) -> c_int {
    value as c_int
}

fn find_value_in_buffer(buffer: *const c_char, size: usize, search_val: c_int) -> c_int {
    let target: c_char = search_val as c_char;
    let result: *const c_void = unsafe {
        libc::memchr(buffer as *const c_void, target as c_int, size)
    };
    if !result.is_null() {
        unsafe { (result as *const c_char).offset_from(buffer) as c_int }
    } else {
        -1
    }
}

fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    for i in 0..size {
        unsafe {
            *buffer.offset(i as isize) = ((seed + i * 7) % 256) as c_char;
        }
    }
}

fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    let mut result: c_double = 0.0;

    if b != 0 {
        result = a as c_double / b as c_double;
    }

    result *= 10.0_f64.powi(c % 10);

    result
}
