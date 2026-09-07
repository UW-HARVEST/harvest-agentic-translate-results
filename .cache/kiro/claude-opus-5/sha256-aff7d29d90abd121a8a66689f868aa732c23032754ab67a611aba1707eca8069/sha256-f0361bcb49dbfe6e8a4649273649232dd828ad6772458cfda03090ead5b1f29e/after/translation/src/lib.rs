// Rust translation of c_src/src/driver.c
//
// Original C library: Copyright 2025 MIT Lincoln Laboratory (MIT-style license,
// see c_src/src/driver.c for the full notice).
//
// The translation preserves the exact public ABI of the C shared library:
//   printLine, printIntLine, bad, good, driver
//
// Output is produced through the platform C library's `printf` so that the
// bytes written, and the stdout buffering/flush semantics, are identical to
// the original C library (including interleaving with any C code in the same
// process that also writes to stdout).

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// void printLine (const char * line)
///
/// Prints `line` followed by a newline; a NULL pointer prints nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            c_printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// void printIntLine (int intNumber)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        c_printf(c"%d\n".as_ptr(), int_number);
    }
}

/// void bad()
///
/// Faithful translation, bug included: the original computes `intOne + intTwo`
/// but discards the result instead of assigning it to `intSum`, so both lines
/// print 0. This is intentional and must NOT be "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    unsafe {
        printIntLine(int_sum);
    }
    // Statement with no effect, exactly as in the C source.
    let _ = int_one.wrapping_add(int_two);
    unsafe {
        printIntLine(int_sum);
    }
}

/// void good()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    unsafe {
        printIntLine(int_sum);
    }
    int_sum = int_one.wrapping_add(int_two);
    unsafe {
        printIntLine(int_sum);
    }
}

/// void driver(void)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver() {
    unsafe {
        printLine(c"Calling good()...".as_ptr());
        good();
        printLine(c"Finished good()".as_ptr());
        printLine(c"Calling bad()...".as_ptr());
        bad();
        printLine(c"Finished bad()".as_ptr());
    }
}
