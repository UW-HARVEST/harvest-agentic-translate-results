use std::os::raw::c_int;

/// Format a f64 like C's printf %e (6 significant digits after decimal, e+XX notation)
fn fmt_e(val: f64) -> String {
    if val == 0.0 {
        return if val.is_sign_negative() {
            "-0.000000e+00".to_string()
        } else {
            "0.000000e+00".to_string()
        };
    }
    if val.is_nan() {
        return "nan".to_string();
    }
    if val.is_infinite() {
        return if val < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    let sign = if val < 0.0 { "-" } else { "" };
    let abs_val = val.abs();
    let exp = abs_val.log10().floor() as i32;
    let mantissa = abs_val / 10.0_f64.powi(exp);
    // Round to 6 decimal places
    let mantissa_rounded = (mantissa * 1e6).round() / 1e6;
    // Handle rounding that pushes mantissa to 10.0
    let (mantissa_final, exp_final) = if mantissa_rounded >= 10.0 {
        (mantissa_rounded / 10.0, exp + 1)
    } else {
        (mantissa_rounded, exp)
    };
    let exp_sign = if exp_final >= 0 { "+" } else { "-" };
    let exp_abs = exp_final.unsigned_abs();
    format!(
        "{}{:.6}e{}{:02}",
        sign, mantissa_final, exp_sign, exp_abs
    )
}

#[inline]
fn convert_double_to_int(value: f64) -> c_int {
    if value.is_nan()
        || value >= (i32::MAX as f64 + 1.0)
        || value < (i32::MIN as f64)
    {
        i32::MIN
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

fn create_numeric_buffer(buffer: &mut [u8], size: c_int, seed: c_int) {
    for i in 0..size {
        buffer[i as usize] = ((seed.wrapping_add(i.wrapping_mul(7))) % 256) as u8;
    }
}

fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
    let mut result: f64 = 0.0;
    if b != 0 {
        result = a as f64 / b as f64;
    }
    result *= 10.0_f64.powi(c % 10);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer = [0u8; 256];

    print!("=== Starting foo() execution ===\n");
    print!("Parameters: {}, {}, {}, {}\n", param1, param2, param3, param4);

    print!("\n--- Integer Negation Test ---\n");
    let negation_test = param1;
    let negation_result = if negation_test != 0 { 1 } else { 0 };
    print!("Original value: {}\n", negation_test);
    print!("After !!negation: {}\n", negation_result);
    result += negation_result * 10;

    let neg_p2 = if param2 != 0 { 1 } else { 0 };
    let neg_p3 = if param3 != 0 { 1 } else { 0 };
    let neg_p4 = if param4 != 0 { 1 } else { 0 };
    print!("Double negation results: {}, {}, {}\n", neg_p2, neg_p3, neg_p4);
    result += neg_p2 + neg_p3 + neg_p4;

    print!("\n--- Double to Int Conversion Test ---\n");

    let large_double = calculate_with_doubles(param1, param2, param3);
    print!("Calculated double value: {}\n", fmt_e(large_double));

    let converted_int = convert_double_to_int(large_double);
    print!("Converted to int (may be UB): {}\n", converted_int);

    let negative_large: f64 = -1.0 * 2.0_f64.powi(40);
    print!("Very large negative double: {}\n", fmt_e(negative_large));
    let converted_neg = convert_double_to_int(negative_large);
    print!("Converted to int (UB likely): {}\n", converted_neg);

    result += (converted_int % 1000) + (converted_neg % 1000);

    print!("\n--- Memchr Search Test ---\n");

    create_numeric_buffer(&mut buffer, 256, param1);

    let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];

    print!("Searching buffer for values...\n");
    for i in 0..4 {
        let pos = find_value_in_buffer(&buffer, search_values[i]);
        if pos >= 0 {
            print!("Found value {} at position {}\n", search_values[i], pos);
            result += pos;
        } else {
            print!("Value {} not found\n", search_values[i]);
        }
    }

    let direct_pos = find_value_in_buffer(&buffer, 100);
    if direct_pos >= 0 {
        print!(
            "Direct memchr found byte 100 at offset: {}\n",
            direct_pos as i64
        );
        result += direct_pos;
    }

    print!("\n--- Combined Feature Test ---\n");
    for i in 0..10 {
        let search_byte = (param1 + i * param2) % 256;
        let pos = find_value_in_buffer(&buffer, search_byte);
        let found_flag = if pos >= 0 { 1 } else { 0 };
        print!("Search {}: byte={}, found={}\n", i, search_byte, found_flag);
        result += found_flag;
    }

    let infinity_val: f64 = f64::INFINITY;
    let nan_val: f64 = f64::NAN;

    print!("\n--- Special Double Values ---\n");
    print!("Converting INFINITY to int: ");
    let inf_as_int = convert_double_to_int(infinity_val);
    print!("{} (undefined behavior)\n", inf_as_int);

    print!("Converting NAN to int: ");
    let nan_as_int = convert_double_to_int(nan_val);
    print!("{} (undefined behavior)\n", nan_as_int);

    print!("\n=== Final Result ===\n");
    print!("Accumulated result: {}\n", result);

    result
}
