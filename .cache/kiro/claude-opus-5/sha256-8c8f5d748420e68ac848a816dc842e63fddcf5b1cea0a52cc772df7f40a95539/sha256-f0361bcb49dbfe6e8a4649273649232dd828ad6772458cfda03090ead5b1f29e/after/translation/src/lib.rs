// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The C library exports exactly four public symbols:
//     printLine, bad, good, driver
// The two `static` helpers (`helperBad`, `helperGood`) stay private, matching
// the C translation unit's internal linkage.
//
// Output is emitted through libc's `printf` rather than Rust's own `println!`
// so that stdout buffering behaviour (and therefore the exact byte stream and
// its interleaving with any other C output in the process) is identical to the
// original library.

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// `printf("%s\n", line)` — the one and only output primitive of the library.
const FMT_STR_NL: &[u8] = b"%s\n\0";

/// Print a NUL-terminated C string followed by a newline, ignoring NULL.
///
/// Mirrors:
/// ```c
/// void printLine(const char *line)
/// {
///     if (line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line != ptr::null() {
        unsafe {
            c_printf(FMT_STR_NL.as_ptr() as *const c_char, line);
        }
    }
}

/// Convenience wrapper for calling `printLine` with a Rust byte literal that
/// already carries its terminating NUL.
#[inline]
fn print_line_lit(lit: &[u8]) {
    debug_assert_eq!(lit.last().copied(), Some(0u8));
    unsafe { printLine(lit.as_ptr() as *const c_char) }
}

/// `static void helperBad()` — never called by the C code either, but kept for
/// fidelity with the original translation unit.
#[allow(dead_code)]
fn helperBad() {
    print_line_lit(b"helperBad()\0");
}

/// `void bad()` — prints its own name and, unlike `good()`, does *not* call its
/// helper. Reproduced exactly as written in the C source.
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    print_line_lit(b"bad()\0");
}

/// `static void helperGood()`
fn helperGood() {
    print_line_lit(b"helperGood()\0");
}

/// `void good()` — prints its own name, then calls its helper.
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    print_line_lit(b"good()\0");
    helperGood();
}

/// `void driver(void)` — the public entry point declared in `include/driver.h`.
#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    print_line_lit(b"Calling good()...\0");
    good();
    print_line_lit(b"Finished good()\0");
    print_line_lit(b"Calling bad()...\0");
    bad();
    print_line_lit(b"Finished bad()\0");
}
