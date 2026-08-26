// Rust translation of c_src/src/lib.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_double, c_int, c_long, c_void};

// ---------------------------------------------------------------------------
// Bindings to the exact same C runtime routines the original translation unit
// used.  Re-using libc's `printf` / `memchr` / `pow` guarantees byte-identical
// output and identical search / floating point semantics.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;

    #[link_name = "memchr"]
    fn c_memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

#[link(name = "m")]
unsafe extern "C" {
    #[link_name = "pow"]
    fn c_pow(x: c_double, y: c_double) -> c_double;
}

/// `printf("literal")` – no variadic arguments.
macro_rules! cprint {
    ($lit:expr) => {{
        unsafe {
            c_printf(concat!($lit, "\0").as_ptr() as *const c_char);
        }
    }};
    ($lit:expr, $($arg:expr),+ $(,)?) => {{
        unsafe {
            c_printf(concat!($lit, "\0").as_ptr() as *const c_char, $($arg),+);
        }
    }};
}

// ---------------------------------------------------------------------------
// Helpers reproducing C's implementation-defined / undefined conversions as
// they actually behave on the reference platform (x86-64 SysV, signed `char`,
// SSE2 `cvttsd2si` for double->int).
// ---------------------------------------------------------------------------

/// Reproduces `(int)value` for an arbitrary `double` on x86-64: out-of-range
/// and NaN inputs yield the "integer indefinite" value `0x80000000`.
#[inline]
fn double_to_int_trunc(value: f64) -> i32 {
    if value.is_nan() {
        return i32::MIN;
    }
    let truncated = value.trunc();
    if truncated >= 2147483648.0 || truncated < -2147483648.0 {
        return i32::MIN;
    }
    truncated as i32
}

// ---------------------------------------------------------------------------
// int convert_double_to_int(double value)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn convert_double_to_int(value: c_double) -> c_int {
    double_to_int_trunc(value) as c_int
}

// ---------------------------------------------------------------------------
// int find_value_in_buffer(const char *buffer, size_t size, int search_val)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    // `char target = (char)search_val;` then `memchr` re-widens it through
    // the default integer promotions before converting to `unsigned char`.
    let target: i8 = search_val as i8;
    let result = unsafe { c_memchr(buffer as *const c_void, target as c_int, size) };
    if !result.is_null() {
        return (result as usize).wrapping_sub(buffer as usize) as isize as c_int;
    }
    -1
}

// ---------------------------------------------------------------------------
// int process_negation(int var1)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    let var2: c_int = if var1 != 0 { 1 } else { 0 };
    var2
}

// ---------------------------------------------------------------------------
// void create_numeric_buffer(char *buffer, int size, int seed)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    let mut i: i32 = 0;
    while i < size {
        // (char)((seed + i * 7) % 256)
        let v = seed.wrapping_add(i.wrapping_mul(7)).wrapping_rem(256);
        unsafe {
            *buffer.offset(i as isize) = v as i8 as c_char;
        }
        i = i.wrapping_add(1);
    }
}

// ---------------------------------------------------------------------------
// double calculate_with_doubles(int a, int b, int c)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn calculate_with_doubles(a: c_int, b: c_int, c: c_int) -> c_double {
    let mut result: f64 = 0.0;

    if b != 0 {
        result = (a as f64) / (b as f64);
    }

    result *= unsafe { c_pow(10.0, (c.wrapping_rem(10)) as f64) };

    result
}

// ---------------------------------------------------------------------------
// int doubleneg(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn doubleneg(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: i32 = 0;
    // `char buffer[256];` – uninitialised in C, fully written by
    // create_numeric_buffer() before any read.
    let mut buffer: [c_char; 256] = [0; 256];
    let bufptr: *mut c_char = buffer.as_mut_ptr();
    let i: i32;

    cprint!("=== Starting foo() execution ===\n");
    cprint!("Parameters: %d, %d, %d, %d\n", param1, param2, param3, param4);

    cprint!("\n--- Integer Negation Test ---\n");
    let negation_test: i32 = param1;
    let negation_result: i32 = if negation_test != 0 { 1 } else { 0 };
    cprint!("Original value: %d\n", negation_test);
    cprint!("After !!negation: %d\n", negation_result);
    result = result.wrapping_add(negation_result.wrapping_mul(10));

    let neg_p2: i32 = if param2 != 0 { 1 } else { 0 };
    let neg_p3: i32 = if param3 != 0 { 1 } else { 0 };
    let neg_p4: i32 = if param4 != 0 { 1 } else { 0 };
    cprint!("Double negation results: %d, %d, %d\n", neg_p2, neg_p3, neg_p4);
    result = result
        .wrapping_add(neg_p2)
        .wrapping_add(neg_p3)
        .wrapping_add(neg_p4);

    cprint!("\n--- Double to Int Conversion Test ---\n");

    let large_double: f64 = calculate_with_doubles(param1, param2, param3);
    cprint!("Calculated double value: %e\n", large_double);

    let converted_int: i32 = convert_double_to_int(large_double);
    cprint!("Converted to int (may be UB): %d\n", converted_int);

    let negative_large: f64 = -1.0 * unsafe { c_pow(2.0, 40.0) };
    cprint!("Very large negative double: %e\n", negative_large);
    let converted_neg: i32 = convert_double_to_int(negative_large);
    cprint!("Converted to int (UB likely): %d\n", converted_neg);

    result = result.wrapping_add(
        converted_int
            .wrapping_rem(1000)
            .wrapping_add(converted_neg.wrapping_rem(1000)),
    );

    cprint!("\n--- Memchr Search Test ---\n");

    unsafe { create_numeric_buffer(bufptr, 256, param1) };

    let search_values: [i32; 4] = [
        param2.wrapping_rem(256),
        param3.wrapping_rem(256),
        param4.wrapping_rem(256),
        42,
    ];
    let num_searches: i32 = search_values.len() as i32;

    cprint!("Searching buffer for values...\n");
    let mut i_loop: i32 = 0;
    while i_loop < num_searches {
        let sv = search_values[i_loop as usize];
        let pos: i32 = unsafe { find_value_in_buffer(bufptr, 256, sv) };
        if pos >= 0 {
            cprint!("Found value %d at position %d\n", sv, pos);
            result = result.wrapping_add(pos);
        } else {
            cprint!("Value %d not found\n", sv);
        }
        i_loop = i_loop.wrapping_add(1);
    }

    let direct_search = unsafe { c_memchr(bufptr as *const c_void, 100, 256) } as *const c_char;
    if !direct_search.is_null() {
        let off = (direct_search as usize).wrapping_sub(bufptr as usize) as isize as i64;
        cprint!(
            "Direct memchr found byte 100 at offset: %ld\n",
            off as c_long,
        );
        result = result.wrapping_add(off as i32);
    }

    cprint!("\n--- Combined Feature Test ---\n");
    let mut j: i32 = 0;
    while j < 10 {
        let search_byte: i32 = param1
            .wrapping_add(j.wrapping_mul(param2))
            .wrapping_rem(256);
        let found = unsafe { c_memchr(bufptr as *const c_void, search_byte, 256) };
        let found_flag: i32 = if !found.is_null() { 1 } else { 0 }; // Double negation on pointer
        cprint!("Search %d: byte=%d, found=%d\n", j, search_byte, found_flag);
        result = result.wrapping_add(found_flag);
        j = j.wrapping_add(1);
    }
    i = j;
    let _ = i;

    let infinity_val: f64 = f64::INFINITY;
    let nan_val: f64 = f64::NAN;

    cprint!("\n--- Special Double Values ---\n");
    cprint!("Converting INFINITY to int: ");
    let inf_as_int: i32 = convert_double_to_int(infinity_val);
    cprint!("%d (undefined behavior)\n", inf_as_int);

    cprint!("Converting NAN to int: ");
    let nan_as_int: i32 = convert_double_to_int(nan_val);
    cprint!("%d (undefined behavior)\n", nan_as_int);

    cprint!("\n=== Final Result ===\n");
    cprint!("Accumulated result: %d\n", result);

    result as c_int
}
