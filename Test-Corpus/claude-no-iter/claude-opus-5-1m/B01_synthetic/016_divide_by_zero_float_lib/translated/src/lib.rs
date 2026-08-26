// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior is preserved to produce byte-identical
// output to the original C implementation.

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// printLine: prints the string followed by a newline if not NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

// printIntLine: prints an int followed by a newline.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, int_number);
    }
}

// bad: performs division `100.0 / data` (in C double precision) and prints
// the integer result. Reproduces the divide-by-zero behavior of the original
// C code (cast of inf/-inf/NaN to int).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: f32) {
    // In C, `100.0` is a double, so the division is done in double precision
    // and then truncated to int.
    let result: c_int = (100.0_f64 / data as f64) as c_int;
    unsafe {
        printIntLine(result);
    }
}

// goodG2B (static in C — private here)
fn good_g2b() {
    let data: f32 = 2.0_f32;
    let result: c_int = (100.0_f64 / data as f64) as c_int;
    unsafe {
        printIntLine(result);
    }
}

// goodB2G (static in C — private here)
fn good_b2g(data: f32) {
    // C uses `fabs((float))` which promotes to double; replicate by casting.
    if (data as f64).abs() > 0.000001_f64 {
        let result: c_int = (100.0_f64 / data as f64) as c_int;
        unsafe {
            printIntLine(result);
        }
    } else {
        let msg = b"This would result in a divide by zero\0".as_ptr() as *const c_char;
        unsafe {
            printLine(msg);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(good_data: f32, bad_data: f32) {
    unsafe {
        printLine(b"Calling good()...\0".as_ptr() as *const c_char);
        good(good_data);
        printLine(b"Finished good()\0".as_ptr() as *const c_char);
        printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
        bad(bad_data);
        printLine(b"Finished bad()\0".as_ptr() as *const c_char);
    }
}
