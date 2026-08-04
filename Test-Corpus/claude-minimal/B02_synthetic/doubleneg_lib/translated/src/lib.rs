// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::os::raw::c_int;

fn convert_double_to_int(value: f64) -> i32 {
    // Mimic C's (int)value cast. Rust's `as i32` is saturating and well-defined,
    // which differs from C's UB but is the closest safe equivalent.
    value as i32
}

fn find_value_in_buffer(buffer: &[u8], size: usize, search_val: i32) -> i32 {
    let target = search_val as u8;
    let limit = size.min(buffer.len());
    for (i, &b) in buffer[..limit].iter().enumerate() {
        if b == target {
            return i as i32;
        }
    }
    -1
}

fn process_negation(var1: i32) -> i32 {
    // C's !!var1
    if var1 != 0 { 1 } else { 0 }
}

fn create_numeric_buffer(buffer: &mut [u8], size: usize, seed: i32) {
    let limit = size.min(buffer.len());
    for i in 0..limit {
        // (seed + i * 7) % 256, then cast to char (i8 in C semantics, but stored as u8)
        // In C, the assignment to char of an int truncates to the low 8 bits.
        let val = seed.wrapping_add((i as i32).wrapping_mul(7));
        let modded = val.rem_euclid(256);
        // Match C's `(char)((seed + i * 7) % 256)` where % can be negative.
        // Actually C's % can yield negative; the cast to char then truncates.
        // To stay close to C: use wrapping cast of the raw modulo result.
        let raw_mod = val % 256;
        let _ = modded; // unused; keep semantics close to C
        buffer[i] = raw_mod as i8 as u8;
    }
}

fn calculate_with_doubles(a: i32, b: i32, c: i32) -> f64 {
    let mut result: f64 = 0.0;

    if b != 0 {
        result = (a as f64) / (b as f64);
    }

    result *= 10.0_f64.powi(c % 10);

    result
}

#[no_mangle]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: i32 = 0;
    let mut buffer: [u8; 256] = [0u8; 256];

    println!("=== Starting foo() execution ===");
    println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

    println!();
    println!("--- Integer Negation Test ---");
    let negation_test = param1;
    let negation_result = process_negation(negation_test);
    println!("Original value: {}", negation_test);
    println!("After !!negation: {}", negation_result);
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2 = process_negation(param2);
    let neg_p3 = process_negation(param3);
    let neg_p4 = process_negation(param4);
    println!("Double negation results: {}, {}, {}", neg_p2, neg_p3, neg_p4);
    result = result
        .wrapping_add(neg_p2)
        .wrapping_add(neg_p3)
        .wrapping_add(neg_p4);

    println!();
    println!("--- Double to Int Conversion Test ---");

    let large_double = calculate_with_doubles(param1, param2, param3);
    println!("Calculated double value: {:e}", large_double);

    let converted_int = convert_double_to_int(large_double);
    println!("Converted to int (may be UB): {}", converted_int);

    let negative_large = -1.0_f64 * 2.0_f64.powi(40);
    println!("Very large negative double: {:e}", negative_large);
    let converted_neg = convert_double_to_int(negative_large);
    println!("Converted to int (UB likely): {}", converted_neg);

    result = result
        .wrapping_add(converted_int % 1000)
        .wrapping_add(converted_neg % 1000);

    println!();
    println!("--- Memchr Search Test ---");

    create_numeric_buffer(&mut buffer, 256, param1);

    // C's % truncates toward zero, matching Rust's % for i32.
    let search_values: [i32; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
    let num_searches = search_values.len();

    println!("Searching buffer for values...");
    for i in 0..num_searches {
        let pos = find_value_in_buffer(&buffer, 256, search_values[i]);
        if pos >= 0 {
            println!("Found value {} at position {}", search_values[i], pos);
            result = result.wrapping_add(pos);
        } else {
            println!("Value {} not found", search_values[i]);
        }
    }

    // memchr for byte 100
    let direct_pos = buffer.iter().position(|&b| b == 100u8);
    if let Some(off) = direct_pos {
        println!("Direct memchr found byte 100 at offset: {}", off);
        result = result.wrapping_add(off as i32);
    }

    println!();
    println!("--- Combined Feature Test ---");
    for i in 0..10i32 {
        let search_byte = param1.wrapping_add(i.wrapping_mul(param2)) % 256;
        let found = buffer.iter().any(|&b| b == (search_byte as u8));
        let found_flag: i32 = if found { 1 } else { 0 };
        println!("Search {}: byte={}, found={}", i, search_byte, found_flag);
        result = result.wrapping_add(found_flag);
    }

    let infinity_val = f64::INFINITY;
    let nan_val = f64::NAN;

    println!();
    println!("--- Special Double Values ---");
    print!("Converting INFINITY to int: ");
    let inf_as_int = convert_double_to_int(infinity_val);
    println!("{} (undefined behavior)", inf_as_int);

    print!("Converting NAN to int: ");
    let nan_as_int = convert_double_to_int(nan_val);
    println!("{} (undefined behavior)", nan_as_int);

    println!();
    println!("=== Final Result ===");
    println!("Accumulated result: {}", result);

    result as c_int
}
