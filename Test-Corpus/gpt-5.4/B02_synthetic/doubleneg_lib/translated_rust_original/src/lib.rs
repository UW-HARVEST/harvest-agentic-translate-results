use libc::{c_char, c_double, c_int, size_t};

fn convert_double_to_int(value: c_double) -> c_int {
    if value.is_nan() {
        0
    } else if value >= c_int::MAX as c_double {
        c_int::MAX
    } else if value <= c_int::MIN as c_double {
        c_int::MIN
    } else {
        value as c_int
    }
}

fn find_value_in_buffer(buffer: &[u8], search_val: c_int) -> c_int {
    let target = search_val as u8;
    match buffer.iter().position(|&b| b == target) {
        Some(pos) => pos as c_int,
        None => -1,
    }
}

fn process_negation(var1: c_int) -> c_int {
    if var1 != 0 { 1 } else { 0 }
}

fn create_numeric_buffer(buffer: &mut [u8], seed: c_int) {
    for (i, byte) in buffer.iter_mut().enumerate() {
        *byte = ((seed + i as c_int * 7).rem_euclid(256)) as u8;
    }
}

fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    let mut result = 0.0;
    if b != 0 {
        result = a as c_double / b as c_double;
    }
    result *= 10.0_f64.powi(c.rem_euclid(10));
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer = [0u8; 256];

    println!("=== Starting foo() execution ===");
    println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

    println!();
    println!("--- Integer Negation Test ---");
    let negation_test = param1;
    let negation_result = process_negation(negation_test);
    println!("Original value: {}", negation_test);
    println!("After !!negation: {}", negation_result);
    result += negation_result * 10;

    let neg_p2 = process_negation(param2);
    let neg_p3 = process_negation(param3);
    let neg_p4 = process_negation(param4);
    println!("Double negation results: {}, {}, {}", neg_p2, neg_p3, neg_p4);
    result += neg_p2 + neg_p3 + neg_p4;

    println!();
    println!("--- Double to Int Conversion Test ---");

    let large_double = calculate_with_doubles(param1, param2, param3);
    println!("Calculated double value: {:e}", large_double);

    let converted_int = convert_double_to_int(large_double);
    println!("Converted to int (may be UB): {}", converted_int);

    let negative_large = -1.0 * 2.0_f64.powi(40);
    println!("Very large negative double: {:e}", negative_large);
    let converted_neg = convert_double_to_int(negative_large);
    println!("Converted to int (UB likely): {}", converted_neg);

    result += converted_int % 1000 + converted_neg % 1000;

    println!();
    println!("--- Memchr Search Test ---");

    create_numeric_buffer(&mut buffer, param1);

    let search_values = [param2 % 256, param3 % 256, param4 % 256, 42];

    println!("Searching buffer for values...");
    for search_value in search_values {
        let pos = find_value_in_buffer(&buffer, search_value);
        if pos >= 0 {
            println!("Found value {} at position {}", search_value, pos);
            result += pos;
        } else {
            println!("Value {} not found", search_value);
        }
    }

    if let Some(offset) = buffer.iter().position(|&b| b == 100) {
        println!("Direct memchr found byte 100 at offset: {}", offset as isize);
        result += offset as c_int;
    }

    println!();
    println!("--- Combined Feature Test ---");
    for i in 0..10 {
        let search_byte = (param1 + i * param2) % 256;
        let found = find_value_in_buffer(&buffer, search_byte) >= 0;
        let found_flag = if found { 1 } else { 0 };
        println!("Search {}: byte={}, found={}", i, search_byte, found_flag);
        result += found_flag;
    }

    let infinity_val = c_double::INFINITY;
    let nan_val = c_double::NAN;

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

    result
}
