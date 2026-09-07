//! Translation of the library's headline entry point, `doubleneg`.
//!
//! Every `printf` call is forwarded to the C library's `printf` with the
//! original format string so that the produced bytes are identical, including
//! `%e` scientific notation and the `%ld` pointer difference.

use core::ffi::{c_int, c_long, c_void};

use crate::buffer::{create_numeric_buffer, find_value_in_buffer};
use crate::conv::convert_double_to_int;
use crate::doubles::calculate_with_doubles;
use crate::ffi;

/// Translation of:
///
/// ```c
/// int doubleneg(int param1, int param2, int param3, int param4);
/// ```
///
/// This is the only function declared in `include/lib.h`, but the whole
/// translation unit is compiled into the shared object, so all six helpers are
/// exported as well.
#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;
    // `char buffer[256];` — left uninitialised in C, but fully written by
    // `create_numeric_buffer` before any read, so zeroing changes nothing.
    let mut buffer = [0i8; 256];
    let mut i: c_int;

    unsafe {
        ffi::printf(c"=== Starting foo() execution ===\n".as_ptr());
        ffi::printf(
            c"Parameters: %d, %d, %d, %d\n".as_ptr(),
            param1,
            param2,
            param3,
            param4,
        );

        ffi::printf(c"\n--- Integer Negation Test ---\n".as_ptr());
    }
    let negation_test: c_int = param1;
    let negation_result: c_int = c_int::from(negation_test != 0);
    unsafe {
        ffi::printf(c"Original value: %d\n".as_ptr(), negation_test);
        ffi::printf(c"After !!negation: %d\n".as_ptr(), negation_result);
    }
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2: c_int = c_int::from(param2 != 0);
    let neg_p3: c_int = c_int::from(param3 != 0);
    let neg_p4: c_int = c_int::from(param4 != 0);
    unsafe {
        ffi::printf(
            c"Double negation results: %d, %d, %d\n".as_ptr(),
            neg_p2,
            neg_p3,
            neg_p4,
        );
    }
    result = result.wrapping_add(neg_p2.wrapping_add(neg_p3).wrapping_add(neg_p4));

    unsafe {
        ffi::printf(c"\n--- Double to Int Conversion Test ---\n".as_ptr());
    }

    let large_double: f64 = calculate_with_doubles(param1, param2, param3);
    unsafe {
        ffi::printf(c"Calculated double value: %e\n".as_ptr(), large_double);
    }

    let converted_int: c_int = convert_double_to_int(large_double);
    unsafe {
        ffi::printf(c"Converted to int (may be UB): %d\n".as_ptr(), converted_int);
    }

    let negative_large: f64 = -1.0 * unsafe { ffi::pow(2.0, 40.0) };
    unsafe {
        ffi::printf(c"Very large negative double: %e\n".as_ptr(), negative_large);
    }
    let converted_neg: c_int = convert_double_to_int(negative_large);
    unsafe {
        ffi::printf(c"Converted to int (UB likely): %d\n".as_ptr(), converted_neg);
    }

    result = result.wrapping_add(
        converted_int
            .wrapping_rem(1000)
            .wrapping_add(converted_neg.wrapping_rem(1000)),
    );

    unsafe {
        ffi::printf(c"\n--- Memchr Search Test ---\n".as_ptr());
    }

    unsafe { create_numeric_buffer(buffer.as_mut_ptr(), 256, param1) };

    let search_values: [c_int; 4] = [
        param2.wrapping_rem(256),
        param3.wrapping_rem(256),
        param4.wrapping_rem(256),
        42,
    ];
    let num_searches: c_int = search_values.len() as c_int;

    unsafe {
        ffi::printf(c"Searching buffer for values...\n".as_ptr());
    }
    i = 0;
    while i < num_searches {
        let pos: c_int =
            unsafe { find_value_in_buffer(buffer.as_ptr(), 256, search_values[i as usize]) };
        if pos >= 0 {
            unsafe {
                ffi::printf(
                    c"Found value %d at position %d\n".as_ptr(),
                    search_values[i as usize],
                    pos,
                );
            }
            result = result.wrapping_add(pos);
        } else {
            unsafe {
                ffi::printf(c"Value %d not found\n".as_ptr(), search_values[i as usize]);
            }
        }
        i = i.wrapping_add(1);
    }

    let direct_search = unsafe { ffi::memchr(buffer.as_ptr() as *const c_void, 100, 256) };
    if !direct_search.is_null() {
        let offset = (direct_search as isize) - (buffer.as_ptr() as isize);
        unsafe {
            ffi::printf(
                c"Direct memchr found byte 100 at offset: %ld\n".as_ptr(),
                offset as c_long,
            );
        }
        result = result.wrapping_add(offset as c_int);
    }

    unsafe {
        ffi::printf(c"\n--- Combined Feature Test ---\n".as_ptr());
    }
    i = 0;
    while i < 10 {
        let search_byte: c_int = param1.wrapping_add(i.wrapping_mul(param2)).wrapping_rem(256);
        let found = unsafe { ffi::memchr(buffer.as_ptr() as *const c_void, search_byte, 256) };
        // Double negation on a pointer.
        let found_flag: c_int = c_int::from(!found.is_null());
        unsafe {
            ffi::printf(
                c"Search %d: byte=%d, found=%d\n".as_ptr(),
                i,
                search_byte,
                found_flag,
            );
        }
        result = result.wrapping_add(found_flag);
        i = i.wrapping_add(1);
    }

    let infinity_val: f64 = f64::INFINITY;
    let nan_val: f64 = f64::NAN;

    unsafe {
        ffi::printf(c"\n--- Special Double Values ---\n".as_ptr());
        ffi::printf(c"Converting INFINITY to int: ".as_ptr());
    }
    let inf_as_int: c_int = convert_double_to_int(infinity_val);
    unsafe {
        ffi::printf(c"%d (undefined behavior)\n".as_ptr(), inf_as_int);

        ffi::printf(c"Converting NAN to int: ".as_ptr());
    }
    let nan_as_int: c_int = convert_double_to_int(nan_val);
    unsafe {
        ffi::printf(c"%d (undefined behavior)\n".as_ptr(), nan_as_int);

        ffi::printf(c"\n=== Final Result ===\n".as_ptr());
        ffi::printf(c"Accumulated result: %d\n".as_ptr(), result);
    }

    result
}
