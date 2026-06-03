// Rust translation of c_src/src/lib.c — must produce byte-identical output.
//
// Strategy:
//   - Public API: `doubleneg` (the only symbol declared in lib.h).
//   - Use libc's `printf`, `pow`, and `memchr` directly via FFI so that
//     formatting and math results are byte-identical to the C version.
//   - Match C's `(int)double` semantics on x86_64 via the `cvttsd2si`
//     intrinsic (`_mm_cvttsd_si32`), which returns INT_MIN (0x80000000)
//     for NaN, +/-Inf, and values out of i32 range — Rust's `as i32` would
//     saturate instead, so we cannot use it for parity.
//   - Use wrapping arithmetic to mirror two's-complement overflow behavior
//     that the C code relies on.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_double, c_int, c_long, c_void};

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{_mm_cvttsd_si32, _mm_set_sd};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn pow(x: c_double, y: c_double) -> c_double;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

// ----- Internal helpers (these do not need C linkage; lib.h only exports
// doubleneg, so the helpers are local to this translation unit, just like
// they effectively are in the C file (no other TU includes them via a
// header). They are kept private to avoid polluting the symbol table. -----

#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: f64) -> c_int {
    // Match `(int)value` on x86_64 (cvttsd2si): returns INT_MIN for NaN,
    // +/-Inf, and values whose truncation does not fit in i32.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        _mm_cvttsd_si32(_mm_set_sd(value))
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // Fallback: replicate cvttsd2si "indefinite integer" semantics
        // (NaN/Inf/out-of-range -> INT_MIN).
        if value.is_nan() {
            return i32::MIN;
        }
        if value >= 2147483648.0 || value < -2147483648.0 {
            return i32::MIN;
        }
        // Truncate toward zero.
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    // C: char target = (char)search_val;  (implementation-defined narrowing)
    //    void *result = memchr(buffer, target, size);
    // memchr internally uses the low 8 bits of its int argument, so the
    // signedness of `char` does not affect which byte is matched.
    let target = search_val as c_char;
    let result = memchr(buffer as *const c_void, target as c_int, size);
    if !result.is_null() {
        return (result as isize - buffer as isize) as c_int;
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    // var2 = !!var1;
    if var1 != 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_numeric_buffer(
    buffer: *mut c_char,
    size: c_int,
    seed: c_int,
) {
    // C: buffer[i] = (char)((seed + i * 7) % 256);
    // Use wrapping arithmetic to mirror C's two's-complement overflow.
    let mut i: c_int = 0;
    while i < size {
        let v = seed.wrapping_add(i.wrapping_mul(7)) % 256;
        *buffer.offset(i as isize) = v as c_char;
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> f64 {
    let mut result: f64 = 0.0;
    if b != 0 {
        result = (a as f64) / (b as f64);
    }
    let exp = (c % 10) as f64;
    unsafe {
        result *= pow(10.0, exp);
    }
    result
}

// Format strings as null-terminated bytes (so we can pass them to printf).
macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: [c_char; 256] = [0; 256];

    unsafe {
        printf(cstr!("=== Starting foo() execution ===\n"));
        printf(
            cstr!("Parameters: %d, %d, %d, %d\n"),
            param1,
            param2,
            param3,
            param4,
        );

        printf(cstr!("\n--- Integer Negation Test ---\n"));
        let negation_test: c_int = param1;
        let negation_result: c_int = if negation_test != 0 { 1 } else { 0 };
        printf(cstr!("Original value: %d\n"), negation_test);
        printf(cstr!("After !!negation: %d\n"), negation_result);
        result = result.wrapping_add(negation_result.wrapping_mul(10));

        let neg_p2: c_int = if param2 != 0 { 1 } else { 0 };
        let neg_p3: c_int = if param3 != 0 { 1 } else { 0 };
        let neg_p4: c_int = if param4 != 0 { 1 } else { 0 };
        printf(
            cstr!("Double negation results: %d, %d, %d\n"),
            neg_p2,
            neg_p3,
            neg_p4,
        );
        result = result
            .wrapping_add(neg_p2)
            .wrapping_add(neg_p3)
            .wrapping_add(neg_p4);

        printf(cstr!("\n--- Double to Int Conversion Test ---\n"));

        let large_double: f64 = calculate_with_doubles(param1, param2, param3);
        printf(cstr!("Calculated double value: %e\n"), large_double);

        let converted_int: c_int = convert_double_to_int(large_double);
        printf(cstr!("Converted to int (may be UB): %d\n"), converted_int);

        let negative_large: f64 = -1.0 * pow(2.0, 40.0);
        printf(cstr!("Very large negative double: %e\n"), negative_large);
        let converted_neg: c_int = convert_double_to_int(negative_large);
        printf(cstr!("Converted to int (UB likely): %d\n"), converted_neg);

        result = result
            .wrapping_add(converted_int % 1000)
            .wrapping_add(converted_neg % 1000);

        printf(cstr!("\n--- Memchr Search Test ---\n"));

        create_numeric_buffer(buffer.as_mut_ptr(), 256, param1);

        let search_values: [c_int; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
        let num_searches: c_int = search_values.len() as c_int;

        printf(cstr!("Searching buffer for values...\n"));
        let mut i: c_int = 0;
        while i < num_searches {
            let pos: c_int =
                find_value_in_buffer(buffer.as_ptr(), 256, search_values[i as usize]);
            if pos >= 0 {
                printf(
                    cstr!("Found value %d at position %d\n"),
                    search_values[i as usize],
                    pos,
                );
                result = result.wrapping_add(pos);
            } else {
                printf(cstr!("Value %d not found\n"), search_values[i as usize]);
            }
            i += 1;
        }

        let direct_search = memchr(buffer.as_ptr() as *const c_void, 100, 256) as *const c_char;
        if !direct_search.is_null() {
            let off: c_long =
                (direct_search as isize - buffer.as_ptr() as isize) as c_long;
            printf(
                cstr!("Direct memchr found byte 100 at offset: %ld\n"),
                off,
            );
            result = result.wrapping_add(off as c_int);
        }

        printf(cstr!("\n--- Combined Feature Test ---\n"));
        let mut i: c_int = 0;
        while i < 10 {
            let search_byte: c_int =
                param1.wrapping_add(i.wrapping_mul(param2)) % 256;
            let found = memchr(buffer.as_ptr() as *const c_void, search_byte, 256);
            let found_flag: c_int = if !found.is_null() { 1 } else { 0 };
            printf(
                cstr!("Search %d: byte=%d, found=%d\n"),
                i,
                search_byte,
                found_flag,
            );
            result = result.wrapping_add(found_flag);
            i += 1;
        }

        let infinity_val: f64 = f64::INFINITY;
        let nan_val: f64 = f64::NAN;

        printf(cstr!("\n--- Special Double Values ---\n"));
        printf(cstr!("Converting INFINITY to int: "));
        let inf_as_int: c_int = convert_double_to_int(infinity_val);
        printf(cstr!("%d (undefined behavior)\n"), inf_as_int);

        printf(cstr!("Converting NAN to int: "));
        let nan_as_int: c_int = convert_double_to_int(nan_val);
        printf(cstr!("%d (undefined behavior)\n"), nan_as_int);

        printf(cstr!("\n=== Final Result ===\n"));
        printf(cstr!("Accumulated result: %d\n"), result);
    }

    result
}
