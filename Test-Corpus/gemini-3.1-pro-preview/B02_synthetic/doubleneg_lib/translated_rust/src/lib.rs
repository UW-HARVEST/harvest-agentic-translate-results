use std::os::raw::c_int;

#[allow(dead_code)]
fn convert_double_to_int(value: f64) -> c_int {
    value as c_int
}

#[allow(dead_code)]
fn find_value_in_buffer(buffer: &[u8], search_val: c_int) -> c_int {
    let target = search_val as u8;
    if let Some(pos) = buffer.iter().position(|&x| x == target) {
        pos as c_int
    } else {
        -1
    }
}

#[allow(dead_code)]
fn process_negation(var1: c_int) -> c_int {
    if var1 != 0 { 1 } else { 0 }
}

#[allow(dead_code)]
fn create_numeric_buffer(buffer: &mut [u8], seed: c_int) {
    for i in 0..buffer.len() {
        buffer[i] = ((seed + (i as c_int) * 7) % 256) as u8;
    }
}

#[allow(dead_code)]
fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
    let mut result = 0.0;

    if b != 0 {
        result = (a as f64) / (b as f64);
    }

    result *= 10.0_f64.powi(c % 10);

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result = 0;
    let mut buffer = [0u8; 256];

    println!("=== Starting foo() execution ===");
    println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

    println!("\n--- Integer Negation Test ---");
    let negation_test = param1;
    let negation_result = if negation_test != 0 { 1 } else { 0 };
    println!("Original value: {}", negation_test);
    println!("After !!negation: {}", negation_result);
    result += negation_result * 10;

    let neg_p2 = if param2 != 0 { 1 } else { 0 };
    let neg_p3 = if param3 != 0 { 1 } else { 0 };
    let neg_p4 = if param4 != 0 { 1 } else { 0 };
    println!("Double negation results: {}, {}, {}", neg_p2, neg_p3, neg_p4);
    result += neg_p2 + neg_p3 + neg_p4;

    println!("\n--- Double to Int Conversion Test ---");

    let large_double = calculate_with_doubles(param1, param2, param3);
    println!("Calculated double value: {:e}", large_double);

    let converted_int = convert_double_to_int(large_double);
    println!("Converted to int (may be UB): {}", converted_int);

    let negative_large = -1.0 * 2.0_f64.powi(40);
    println!("Very large negative double: {:e}", negative_large);
    let converted_neg = convert_double_to_int(negative_large);
    println!("Converted to int (UB likely): {}", converted_neg);

    result += (converted_int % 1000) + (converted_neg % 1000);

    println!("\n--- Memchr Search Test ---");

    create_numeric_buffer(&mut buffer, param1);

    let search_values = [param2 % 256, param3 % 256, param4 % 256, 42];

    println!("Searching buffer for values...");
    for &val in &search_values {
        let pos = find_value_in_buffer(&buffer, val);
        if pos >= 0 {
            println!("Found value {} at position {}", val, pos);
            result += pos;
        } else {
            println!("Value {} not found", val);
        }
    }

    let direct_search = buffer.iter().position(|&x| x == 100);
    if let Some(pos) = direct_search {
        println!("Direct memchr found byte 100 at offset: {}", pos);
        result += pos as c_int;
    }

    println!("\n--- Combined Feature Test ---");
    for i in 0..10 {
        let search_byte_int = (param1 + i * param2) % 256;
        let search_byte = search_byte_int as u8;
        let found = buffer.iter().position(|&x| x == search_byte);
        let found_flag = if found.is_some() { 1 } else { 0 };
        println!("Search {}: byte={}, found={}", i, search_byte_int, found_flag);
        result += found_flag;
    }

    let infinity_val = f64::INFINITY;
    let nan_val = f64::NAN;

    println!("\n--- Special Double Values ---");
    print!("Converting INFINITY to int: ");
    let inf_as_int = convert_double_to_int(infinity_val);
    println!("{} (undefined behavior)", inf_as_int);

    print!("Converting NAN to int: ");
    let nan_as_int = convert_double_to_int(nan_val);
    println!("{} (undefined behavior)", nan_as_int);

    println!("\n=== Final Result ===");
    println!("Accumulated result: {}", result);

    result
}
