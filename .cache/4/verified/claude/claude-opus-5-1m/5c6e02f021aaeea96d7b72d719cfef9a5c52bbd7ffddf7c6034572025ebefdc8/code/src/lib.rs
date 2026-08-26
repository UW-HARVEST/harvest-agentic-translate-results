// Translated from c_src/src/main.c -- C ABI export surface.
//
// This crate is compiled as a `cdylib` so that the translated code can be
// exercised through exactly the same dynamic-symbol surface that
// `c_src/src/main.c` exposes when it is compiled as a shared library:
//
//     bad, good, main, printIntLine, printLine
//
// The wrappers below are the ONLY place `#[no_mangle]` appears; the actual
// translation lives in `translated.rs`, which is shared with the `driver`
// binary.

pub mod translated;

use std::ffi::c_char;
use std::ffi::c_int;

pub use translated::{bad as rust_bad, c_main, good as rust_good, print_int_line, print_line, Scanner};

/// `void printLine (const char * line)`
///
/// # Safety
/// `line` must either be NULL or point to a NUL-terminated C string.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        // C: `if (line != NULL)` -- nothing is printed for NULL.
        return;
    }
    let bytes = std::ffi::CStr::from_ptr(line).to_bytes();
    translated::print_line(Some(bytes));
}

/// `void printIntLine (int intNumber)`
#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    translated::print_int_line(intNumber);
}

/// `void bad()`
#[no_mangle]
pub extern "C" fn bad() {
    translated::bad();
}

/// `void good()`
#[no_mangle]
pub extern "C" fn good() {
    translated::good();
}

/// `int main()`
///
/// `c_src/src/main.c` compiled as a shared object exports `main` as an ordinary
/// function; the cdylib mirrors that so the symbol surfaces match and the entry
/// point can be driven through `dlopen`/`dlsym` in the differential tests.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    translated::c_main()
}
