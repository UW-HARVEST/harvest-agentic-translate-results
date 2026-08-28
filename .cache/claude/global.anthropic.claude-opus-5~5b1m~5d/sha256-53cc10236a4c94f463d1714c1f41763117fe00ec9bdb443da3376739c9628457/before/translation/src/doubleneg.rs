//! Translation of `doubleneg` from `c_src/src/lib.c`.
//!
//! This is the library's headline entry point (the only one declared in
//! `include/lib.h`). It is a driver that prints a running commentary while
//! exercising the other helpers, so the translation reproduces every `printf`
//! call, in order, with the identical format string -- output is produced by the
//! very same libc `printf` the C code used.

use core::ffi::c_char;
use core::ffi::c_int;
use core::ffi::c_long;

use crate::buffer;
use crate::cvt;
use crate::dmath;
use crate::ffi;
use crate::ffi::cstr;

/// C: `int doubleneg(int param1, int param2, int param3, int param4)`
#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;
    // C: `char buffer[256];` -- declared uninitialised, but
    // `create_numeric_buffer` below writes all 256 bytes before any read.
    let mut buf: [c_char; 256] = [0; 256];

    unsafe {
        ffi::printf(cstr!("=== Starting foo() execution ===\n"));
        ffi::printf(
            cstr!("Parameters: %d, %d, %d, %d\n"),
            param1,
            param2,
            param3,
            param4,
        );

        ffi::printf(cstr!("\n--- Integer Negation Test ---\n"));
        let negation_test: c_int = param1;
        let negation_result: c_int = c_int::from(negation_test != 0);
        ffi::printf(cstr!("Original value: %d\n"), negation_test);
        ffi::printf(cstr!("After !!negation: %d\n"), negation_result);
        result = result.wrapping_add(negation_result.wrapping_mul(10));

        let neg_p2: c_int = c_int::from(param2 != 0);
        let neg_p3: c_int = c_int::from(param3 != 0);
        let neg_p4: c_int = c_int::from(param4 != 0);
        ffi::printf(
            cstr!("Double negation results: %d, %d, %d\n"),
            neg_p2,
            neg_p3,
            neg_p4,
        );
        result = result
            .wrapping_add(neg_p2)
            .wrapping_add(neg_p3)
            .wrapping_add(neg_p4);

        ffi::printf(cstr!("\n--- Double to Int Conversion Test ---\n"));

        let large_double = dmath::calculate(param1, param2, param3);
        ffi::printf(cstr!("Calculated double value: %e\n"), large_double);

        let converted_int = cvt::convert(large_double);
        ffi::printf(cstr!("Converted to int (may be UB): %d\n"), converted_int);

        let negative_large = -1.0_f64 * ffi::pow(2.0, 40.0);
        ffi::printf(cstr!("Very large negative double: %e\n"), negative_large);
        let converted_neg = cvt::convert(negative_large);
        ffi::printf(cstr!("Converted to int (UB likely): %d\n"), converted_neg);

        // Both remainders truncate toward zero; `INT_MIN % 1000` is `-648`.
        result = result
            .wrapping_add(converted_int % 1000)
            .wrapping_add(converted_neg % 1000);

        ffi::printf(cstr!("\n--- Memchr Search Test ---\n"));

        buffer::create_numeric_buffer(buf.as_mut_ptr(), 256, param1);

        let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
        let num_searches = search_values.len();

        ffi::printf(cstr!("Searching buffer for values...\n"));
        for i in 0..num_searches {
            let pos = buffer::find_value_in_buffer(buf.as_ptr(), 256, search_values[i]);
            if pos >= 0 {
                ffi::printf(
                    cstr!("Found value %d at position %d\n"),
                    search_values[i],
                    pos,
                );
                result = result.wrapping_add(pos);
            } else {
                ffi::printf(cstr!("Value %d not found\n"), search_values[i]);
            }
        }

        // C: `char *direct_search = (char*)memchr(buffer, 100, 256);`
        let direct_search = buffer::memchr(buf.as_ptr(), 100, 256);
        if let Some(offset) = direct_search {
            ffi::printf(
                cstr!("Direct memchr found byte 100 at offset: %ld\n"),
                offset as c_long,
            );
            result = result.wrapping_add(offset as c_int);
        }

        ffi::printf(cstr!("\n--- Combined Feature Test ---\n"));
        for i in 0..10_i32 {
            let search_byte: c_int = param1.wrapping_add(i.wrapping_mul(param2)) % 256;
            let found = buffer::memchr(buf.as_ptr(), search_byte as u8, 256);
            // C: `int found_flag = !!found;` -- double negation on a pointer.
            let found_flag: c_int = c_int::from(found.is_some());
            ffi::printf(
                cstr!("Search %d: byte=%d, found=%d\n"),
                i,
                search_byte,
                found_flag,
            );
            result = result.wrapping_add(found_flag);
        }

        let infinity_val = f64::INFINITY;
        let nan_val = f64::NAN;

        ffi::printf(cstr!("\n--- Special Double Values ---\n"));
        ffi::printf(cstr!("Converting INFINITY to int: "));
        let inf_as_int = cvt::convert(infinity_val);
        ffi::printf(cstr!("%d (undefined behavior)\n"), inf_as_int);

        ffi::printf(cstr!("Converting NAN to int: "));
        let nan_as_int = cvt::convert(nan_val);
        ffi::printf(cstr!("%d (undefined behavior)\n"), nan_as_int);

        ffi::printf(cstr!("\n=== Final Result ===\n"));
        ffi::printf(cstr!("Accumulated result: %d\n"), result);
    }

    result
}
