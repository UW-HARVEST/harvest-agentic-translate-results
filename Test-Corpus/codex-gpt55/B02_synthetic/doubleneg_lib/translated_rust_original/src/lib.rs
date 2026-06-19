use std::ffi::{c_char, c_double, c_int, c_long, c_void};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

#[link(name = "m")]
unsafe extern "C" {
    fn pow(x: c_double, y: c_double) -> c_double;
}

#[inline]
fn c_bool(value: c_int) -> c_int {
    if value != 0 { 1 } else { 0 }
}

#[inline]
fn c_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

#[inline]
fn c_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[inline]
fn c_rem(a: c_int, b: c_int) -> c_int {
    a % b
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn double_to_c_int(value: c_double) -> c_int {
    let out: c_int;
    unsafe {
        core::arch::asm!(
            "cvttsd2si {out:e}, {value}",
            value = in(xmm_reg) value,
            out = lateout(reg) out,
            options(nostack, nomem, preserves_flags)
        );
    }
    out
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
fn double_to_c_int(value: c_double) -> c_int {
    if !value.is_finite() || value < c_int::MIN as c_double || value > c_int::MAX as c_double {
        c_int::MIN
    } else {
        value.trunc() as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: c_double) -> c_int {
    double_to_c_int(value)
}

#[unsafe(no_mangle)]
pub extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    let target = search_val as c_char;
    let result = unsafe { memchr(buffer.cast::<c_void>(), target as c_int, size) };
    if !result.is_null() {
        return unsafe { result.cast::<c_char>().offset_from(buffer) as c_int };
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    c_bool(var1)
}

#[unsafe(no_mangle)]
pub extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    for i in 0..size {
        let value = c_rem(c_add(seed, c_mul(i, 7)), 256) as c_char;
        unsafe {
            *buffer.offset(i as isize) = value;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    let mut result = 0.0;

    if b != 0 {
        result = (a as c_double) / (b as c_double);
    }

    unsafe {
        result *= pow(10.0, c_rem(c, 10) as c_double);
    }

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

        printf(c"\n--- Integer Negation Test ---\n".as_ptr());
    }
    let negation_test = param1;
    let negation_result = c_bool(negation_test);
    unsafe {
        printf(c"Original value: %d\n".as_ptr(), negation_test);
        printf(c"After !!negation: %d\n".as_ptr(), negation_result);
    }
    result = c_add(result, c_mul(negation_result, 10));

    let neg_p2 = c_bool(param2);
    let neg_p3 = c_bool(param3);
    let neg_p4 = c_bool(param4);
    unsafe {
        printf(
            c"Double negation results: %d, %d, %d\n".as_ptr(),
            neg_p2,
            neg_p3,
            neg_p4,
        );
    }
    result = c_add(result, c_add(c_add(neg_p2, neg_p3), neg_p4));

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

    let negative_large = -1.0 * unsafe { pow(2.0, 40.0) };
    unsafe {
        printf(c"Very large negative double: %e\n".as_ptr(), negative_large);
    }
    let converted_neg = convert_double_to_int(negative_large);
    unsafe {
        printf(c"Converted to int (UB likely): %d\n".as_ptr(), converted_neg);
    }

    result = c_add(
        result,
        c_add(c_rem(converted_int, 1000), c_rem(converted_neg, 1000)),
    );

    unsafe {
        printf(c"\n--- Memchr Search Test ---\n".as_ptr());
    }

    create_numeric_buffer(buffer.as_mut_ptr(), 256, param1);

    let search_values = [
        c_rem(param2, 256),
        c_rem(param3, 256),
        c_rem(param4, 256),
        42,
    ];

    unsafe {
        printf(c"Searching buffer for values...\n".as_ptr());
    }
    for search_value in search_values {
        let pos = find_value_in_buffer(buffer.as_ptr(), 256, search_value);
        if pos >= 0 {
            unsafe {
                printf(
                    c"Found value %d at position %d\n".as_ptr(),
                    search_value,
                    pos,
                );
            }
            result = c_add(result, pos);
        } else {
            unsafe {
                printf(c"Value %d not found\n".as_ptr(), search_value);
            }
        }
    }

    let direct_search = unsafe { memchr(buffer.as_ptr().cast::<c_void>(), 100, 256) };
    if !direct_search.is_null() {
        let offset = unsafe { direct_search.cast::<c_char>().offset_from(buffer.as_ptr()) };
        unsafe {
            printf(
                c"Direct memchr found byte 100 at offset: %ld\n".as_ptr(),
                offset as c_long,
            );
        }
        result = c_add(result, offset as c_int);
    }

    unsafe {
        printf(c"\n--- Combined Feature Test ---\n".as_ptr());
    }
    for i in 0..10 {
        let search_byte = c_rem(c_add(param1, c_mul(i, param2)), 256);
        let found = unsafe { memchr(buffer.as_ptr().cast::<c_void>(), search_byte, 256) };
        let found_flag = if !found.is_null() { 1 } else { 0 };
        unsafe {
            printf(
                c"Search %d: byte=%d, found=%d\n".as_ptr(),
                i,
                search_byte,
                found_flag,
            );
        }
        result = c_add(result, found_flag);
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
