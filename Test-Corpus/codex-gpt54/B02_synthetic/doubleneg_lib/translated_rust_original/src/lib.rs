use std::ffi::{c_char, c_double, c_int, c_long};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(x: c_double, y: c_double) -> c_double;
}

fn convert_double_to_int(value: c_double) -> c_int {
    hardware_double_to_int(value)
}

#[cfg(target_arch = "x86_64")]
fn hardware_double_to_int(value: c_double) -> c_int {
    use std::arch::x86_64::{_mm_cvttsd_si32, _mm_set_sd};

    unsafe { _mm_cvttsd_si32(_mm_set_sd(value)) }
}

#[cfg(target_arch = "x86")]
fn hardware_double_to_int(value: c_double) -> c_int {
    use std::arch::x86::{_mm_cvttsd_si32, _mm_set_sd};

    unsafe { _mm_cvttsd_si32(_mm_set_sd(value)) }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
fn hardware_double_to_int(value: c_double) -> c_int {
    value as c_int
}

fn find_value_in_buffer(buffer: &[u8], search_val: c_int) -> c_int {
    let target = (search_val as i8) as u8;
    match buffer.iter().position(|&byte| byte == target) {
        Some(pos) => pos as c_int,
        None => -1,
    }
}

fn process_negation(var1: c_int) -> c_int {
    if var1 != 0 { 1 } else { 0 }
}

fn create_numeric_buffer(buffer: &mut [u8; 256], seed: c_int) {
    for (i, byte) in buffer.iter_mut().enumerate() {
        let index = i as c_int;
        let value = seed.wrapping_add(index.wrapping_mul(7)) % 256;
        *byte = (value as i8) as u8;
    }
}

fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    let mut result = 0.0;

    if b != 0 {
        result = (a as c_double) / (b as c_double);
    }

    unsafe {
        result *= pow(10.0, (c % 10) as c_double);
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer = [0u8; 256];

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
    let negation_result = process_negation(negation_test);

    unsafe {
        printf(c"Original value: %d\n".as_ptr(), negation_test);
        printf(c"After !!negation: %d\n".as_ptr(), negation_result);
    }
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2 = process_negation(param2);
    let neg_p3 = process_negation(param3);
    let neg_p4 = process_negation(param4);

    unsafe {
        printf(
            c"Double negation results: %d, %d, %d\n".as_ptr(),
            neg_p2,
            neg_p3,
            neg_p4,
        );
        printf(c"\n--- Double to Int Conversion Test ---\n".as_ptr());
    }
    result = result.wrapping_add(neg_p2);
    result = result.wrapping_add(neg_p3);
    result = result.wrapping_add(neg_p4);

    let large_double = calculate_with_doubles(param1, param2, param3);
    unsafe {
        printf(c"Calculated double value: %e\n".as_ptr(), large_double);
    }

    let converted_int = convert_double_to_int(large_double);
    unsafe {
        printf(
            c"Converted to int (may be UB): %d\n".as_ptr(),
            converted_int,
        );
    }

    let negative_large = unsafe { -1.0 * pow(2.0, 40.0) };
    unsafe {
        printf(c"Very large negative double: %e\n".as_ptr(), negative_large);
    }
    let converted_neg = convert_double_to_int(negative_large);
    unsafe {
        printf(
            c"Converted to int (UB likely): %d\n".as_ptr(),
            converted_neg,
        );
    }

    result = result.wrapping_add(converted_int % 1000);
    result = result.wrapping_add(converted_neg % 1000);

    unsafe {
        printf(c"\n--- Memchr Search Test ---\n".as_ptr());
    }

    create_numeric_buffer(&mut buffer, param1);

    let search_values = [param2 % 256, param3 % 256, param4 % 256, 42];

    unsafe {
        printf(c"Searching buffer for values...\n".as_ptr());
    }
    for search_value in search_values {
        let pos = find_value_in_buffer(&buffer, search_value);
        if pos >= 0 {
            unsafe {
                printf(
                    c"Found value %d at position %d\n".as_ptr(),
                    search_value,
                    pos,
                );
            }
            result = result.wrapping_add(pos);
        } else {
            unsafe {
                printf(c"Value %d not found\n".as_ptr(), search_value);
            }
        }
    }

    if let Some(pos) = buffer.iter().position(|&byte| byte == 100) {
        unsafe {
            printf(
                c"Direct memchr found byte 100 at offset: %ld\n".as_ptr(),
                pos as c_long,
            );
        }
        result = result.wrapping_add(pos as c_int);
    }

    unsafe {
        printf(c"\n--- Combined Feature Test ---\n".as_ptr());
    }
    for i in 0..10 {
        let search_byte = param1.wrapping_add((i as c_int).wrapping_mul(param2)) % 256;
        let found_flag = if find_value_in_buffer(&buffer, search_byte) >= 0 {
            1
        } else {
            0
        };
        unsafe {
            printf(
                c"Search %d: byte=%d, found=%d\n".as_ptr(),
                i as c_int,
                search_byte,
                found_flag,
            );
        }
        result = result.wrapping_add(found_flag);
    }

    let infinity_val = c_double::INFINITY;
    let nan_val = c_double::NAN;

    unsafe {
        printf(c"\n--- Special Double Values ---\n".as_ptr());
        printf(c"Converting INFINITY to int: ".as_ptr());
    }
    let inf_as_int = convert_double_to_int(infinity_val);
    unsafe {
        printf(c"%d (undefined behavior)\n".as_ptr(), inf_as_int);
        printf(c"Converting NAN to int: ".as_ptr());
    }
    let nan_as_int = convert_double_to_int(nan_val);
    unsafe {
        printf(c"%d (undefined behavior)\n".as_ptr(), nan_as_int);
        printf(c"\n=== Final Result ===\n".as_ptr());
        printf(c"Accumulated result: %d\n".as_ptr(), result);
    }

    result
}
