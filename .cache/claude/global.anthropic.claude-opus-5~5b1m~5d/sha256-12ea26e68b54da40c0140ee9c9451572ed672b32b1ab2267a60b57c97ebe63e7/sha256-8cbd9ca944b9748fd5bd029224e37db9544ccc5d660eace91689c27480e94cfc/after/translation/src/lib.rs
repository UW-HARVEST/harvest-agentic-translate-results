// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The original C library is built as a shared object exporting the symbols
// `printLine`, `printIntLine`, `bad`, `good` and `driver`.  All of them are
// re-exported here with identical signatures and identical observable output.
//
// Output is produced through the platform C `printf` so that the stdio stream,
// its buffering mode and the resulting byte stream are exactly the same as for
// the original library.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string `"%s\n"` (NUL terminated), as used by the C source.
const FMT_STR: &[u8] = b"%s\n\0";
/// Format string `"%d\n"` (NUL terminated), as used by the C source.
const FMT_INT: &[u8] = b"%d\n\0";

/// `void printLine(const char * line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(FMT_STR.as_ptr() as *const c_char, line);
    }
}

/// `void printIntLine(int intNumber)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    printf(FMT_INT.as_ptr() as *const c_char, intNumber);
}

/// `void bad(void)`
///
/// Faithfully reproduces the original CWE-482-style defect: the result of
/// `intOne + intTwo` is computed and thrown away, so `intSum` stays 0 and the
/// same value is printed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    printIntLine(int_sum);
    let _ = int_one.wrapping_add(int_two); // result intentionally discarded
    printIntLine(int_sum);
}

/// `void good(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    printIntLine(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

/// `void driver(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver() {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
