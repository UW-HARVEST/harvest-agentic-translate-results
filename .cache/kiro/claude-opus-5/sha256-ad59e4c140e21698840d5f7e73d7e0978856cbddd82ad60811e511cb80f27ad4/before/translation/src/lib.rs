// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The original C library is compiled by CMake into a single shared object that
// exports exactly five public symbols:
//
//     printLine, printIntLine, bad, good, driver
//
// `goodG2B` and `goodB2G` are `static` in the C source, so they are *not*
// exported; they are kept private here as well.
//
// Behavioural notes (deliberately reproduced, not "fixed"):
//
//  * `bad()` performs `100.0 / data` with no guard at all, so a zero (or
//    denormal/tiny) `data` produces an infinite/huge double that is then cast to
//    `int`. That cast is undefined behaviour in C; on x86-64 the compiler emits
//    `cvttsd2si`, which returns the "integer indefinite" value `0x80000000`
//    (`-2147483648`) for NaN and for anything outside the `int` range. The C
//    reference binary prints `-2147483648` in those cases, so `to_c_int()`
//    below reproduces that instead of Rust's saturating `as` cast.
//  * `goodB2G()` guards with `fabs(data) > 0.000001`, which is *not* the same as
//    a divide-by-zero check: values such as 5e-07 fall into the "would divide by
//    zero" branch even though the division is finite, and NaN also lands there
//    (all NaN comparisons are false). Left exactly as written.
//  * The division is done in `double`, not `float`: the `float` argument is
//    promoted to `double` and divided into the `double` literal `100.0`.
//
// All printing goes through libc's `printf` so that this library shares the C
// stdio buffer and produces byte-identical, identically-ordered output.

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_float;
use std::ffi::c_int;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%s\n"` — the format string used by the C `printLine`.
const FMT_STR_NL: &[u8] = b"%s\n\0";
/// `"%d\n"` — the format string used by the C `printIntLine`.
const FMT_INT_NL: &[u8] = b"%d\n\0";

/// The message emitted by the C `goodB2G` when its guard rejects `data`.
const DIVIDE_BY_ZERO_MSG: &[u8] = b"This would result in a divide by zero\0";

/// Truncating `double` -> `int` conversion with x86-64 `cvttsd2si` semantics.
///
/// C's `(int)some_double` is undefined when the truncated value does not fit in
/// an `int`; on x86-64 the hardware yields the "integer indefinite" value
/// `INT_MIN`. Rust's `as` cast instead saturates, so it cannot be used directly
/// if the output is to match the C library byte for byte.
fn to_c_int(value: f64) -> c_int {
    // NaN -> integer indefinite.
    if value.is_nan() {
        return c_int::MIN;
    }
    // Truncation toward zero must land within [-2^31, 2^31 - 1]. Anything at or
    // above 2^31, or at or below -(2^31 + 1), is out of range. Note that values
    // in (-2147483649.0, -2147483648.0] truncate to exactly INT_MIN and are
    // therefore in range.
    if value >= 2_147_483_648.0 || value <= -2_147_483_649.0 {
        return c_int::MIN;
    }
    value as c_int
}

/// C: `void printLine(const char * line)`
///
/// Prints `line` followed by a newline, but only when the pointer is non-NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(FMT_STR_NL.as_ptr() as *const c_char, line);
    }
}

/// C: `void printIntLine(int intNumber)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    printf(FMT_INT_NL.as_ptr() as *const c_char, intNumber);
}

/// C: `void bad(float data)`
///
/// Divides by `data` with no validation whatsoever — the flaw under test.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_float) {
    let result: c_int = to_c_int(100.0f64 / data as f64);
    printIntLine(result);
}

/// C: `static void goodG2B(void)`
///
/// Uses a hard-coded, known-safe divisor, so it always prints 50.
unsafe fn goodG2B() {
    let data: c_float = 2.0f32;
    let result: c_int = to_c_int(100.0f64 / data as f64);
    printIntLine(result);
}

/// C: `static void goodB2G(float data)`
///
/// Screens `data` with `fabs(data) > 0.000001` before dividing.
unsafe fn goodB2G(data: c_float) {
    if (data as f64).abs() > 0.000001 {
        let result: c_int = to_c_int(100.0f64 / data as f64);
        printIntLine(result);
    } else {
        printLine(DIVIDE_BY_ZERO_MSG.as_ptr() as *const c_char);
    }
}

/// C: `void good(float data)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_float) {
    goodG2B();
    goodB2G(data);
}

/// C: `void driver(float goodData, float badData)`
///
/// The public entry point declared in `include/driver.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(goodData: c_float, badData: c_float) {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good(goodData);
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad(badData);
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
