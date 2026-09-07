use std::ffi::{c_char, c_double, c_int, c_long, c_void};

#[link(name = "c")]
unsafe extern "C" {
    fn memchr(buffer: *const c_void, byte: c_int, size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(base: c_double, exponent: c_double) -> c_double;
}

#[inline]
fn platform_double_to_int(value: c_double) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let result: c_int;
        unsafe {
            core::arch::asm!(
                "cvttsd2si {result:e}, {value}",
                result = out(reg) result,
                value = in(xmm_reg) value,
                options(pure, nomem, nostack)
            );
        }
        result
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: c_double) -> c_int {
    platform_double_to_int(value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    let target = search_val as c_char;
    let result = unsafe { memchr(buffer.cast(), target as c_int, size) };

    if result.is_null() {
        -1
    } else {
        (result as usize).wrapping_sub(buffer as usize) as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    (var1 != 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    for i in 0..size {
        let value = seed.wrapping_add(i.wrapping_mul(7)) % 256;
        unsafe {
            buffer.add(i as usize).write(value as c_char);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    let mut result = 0.0;

    if b != 0 {
        result = a as c_double / b as c_double;
    }

    unsafe {
        result *= pow(10.0, (c % 10) as c_double);
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer = [0 as c_char; 256];

    unsafe {
        printf(c"=== Starting foo() execution ===\n".as_ptr());
        printf(
            c"Parameters: %d, %d, %d, %d\n".as_ptr(),
            param1,
            param2,
            param3,
            param4,
        );
    }

    unsafe {
        printf(c"\n--- Integer Negation Test ---\n".as_ptr());
    }
    let negation_test = param1;
    let negation_result = (negation_test != 0) as c_int;
    unsafe {
        printf(c"Original value: %d\n".as_ptr(), negation_test);
        printf(c"After !!negation: %d\n".as_ptr(), negation_result);
    }
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2 = (param2 != 0) as c_int;
    let neg_p3 = (param3 != 0) as c_int;
    let neg_p4 = (param4 != 0) as c_int;
    unsafe {
        printf(
            c"Double negation results: %d, %d, %d\n".as_ptr(),
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
        printf(c"\n--- Double to Int Conversion Test ---\n".as_ptr());
    }

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

    let negative_large = -unsafe { pow(2.0, 40.0) };
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

    result = result
        .wrapping_add(converted_int % 1000)
        .wrapping_add(converted_neg % 1000);

    unsafe {
        printf(c"\n--- Memchr Search Test ---\n".as_ptr());
        create_numeric_buffer(buffer.as_mut_ptr(), 256, param1);
    }

    let search_values = [param2 % 256, param3 % 256, param4 % 256, 42];

    unsafe {
        printf(c"Searching buffer for values...\n".as_ptr());
    }
    for search_value in search_values {
        let pos = unsafe { find_value_in_buffer(buffer.as_ptr(), buffer.len(), search_value) };
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

    let direct_search = unsafe { memchr(buffer.as_ptr().cast(), 100, buffer.len()) };
    if !direct_search.is_null() {
        let offset = (direct_search as usize).wrapping_sub(buffer.as_ptr() as usize);
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
    for i in 0_i32..10 {
        let search_byte = param1.wrapping_add(i.wrapping_mul(param2)) % 256;
        let found = unsafe { memchr(buffer.as_ptr().cast(), search_byte, buffer.len()) };
        let found_flag = (!found.is_null()) as c_int;
        unsafe {
            printf(
                c"Search %d: byte=%d, found=%d\n".as_ptr(),
                i,
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
