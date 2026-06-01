// Translation of c_src/src/driver.c to Rust.
// The original C is licensed under the MIT-like license (Copyright 2025 MIT
// Lincoln Laboratory) reproduced in the C source file.

use core::ffi::{c_char, c_float, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fabs(x: f64) -> f64;
}

/// Reproduces `printf("%s\n", line);` when `line != NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // "%s\n" with NUL terminator.
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        printf(fmt, line);
    }
}

/// Reproduces `printf("%d\n", intNumber);`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    printf(fmt, int_number);
}

/// Convert a double to int matching the C `(int)d` cast semantics on the
/// target platform (truncation; out-of-range / NaN is UB in C, but on x86
/// produces the "indefinite integer value" `INT_MIN`).
#[inline]
fn double_to_int_c(value: f64) -> c_int {
    // Use `to_int_unchecked` so that we mirror C's truncation-with-UB
    // behavior (rather than Rust's saturating `as` cast). This is safe
    // for the well-defined (in-range, finite) inputs that the C program
    // exercises during normal execution; for out-of-range inputs we
    // intentionally match the platform's native float-to-int conversion
    // instruction, matching the C reference.
    unsafe { f64::to_int_unchecked::<c_int>(value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_float) {
    // In C, `100.0 / data` promotes `data` to `double` because `100.0` is a
    // double literal; the division and cast to int therefore happen in f64.
    let result: c_int = double_to_int_c(100.0_f64 / (data as f64));
    printIntLine(result);
}

fn good_g2b() {
    let data: f32 = 2.0_f32;
    {
        let result: c_int = double_to_int_c(100.0_f64 / (data as f64));
        unsafe { printIntLine(result); }
    }
}

fn good_b2g(data: c_float) {
    if unsafe { fabs(data as f64) } > 0.000001 {
        let result: c_int = double_to_int_c(100.0_f64 / (data as f64));
        unsafe { printIntLine(result); }
    } else {
        let msg = b"This would result in a divide by zero\0".as_ptr() as *const c_char;
        unsafe { printLine(msg); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    let calling_good = b"Calling good()...\0".as_ptr() as *const c_char;
    let finished_good = b"Finished good()\0".as_ptr() as *const c_char;
    let calling_bad = b"Calling bad()...\0".as_ptr() as *const c_char;
    let finished_bad = b"Finished bad()\0".as_ptr() as *const c_char;

    printLine(calling_good);
    good(good_data);
    printLine(finished_good);
    printLine(calling_bad);
    bad(bad_data);
    printLine(finished_bad);
}
