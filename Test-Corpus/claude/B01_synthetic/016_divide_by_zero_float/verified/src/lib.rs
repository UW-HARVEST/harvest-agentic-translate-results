// C-ABI surface of the translated `driver` translation unit.
//
// The original C file (c_src/src/main.c) is compiled into an executable, but
// every non-`static` function in it is an ordinary external symbol. Building
// that translation unit as a shared object exports exactly:
//
//     printLine, printIntLine, bad, good, main
//
// (`goodG2B` and `goodB2G` are `static` and therefore have internal linkage,
// so they are deliberately NOT exported here either.)
//
// This crate re-exports the same five symbols with the same C ABI so that an
// external caller -- e.g. a differential test using `libloading` -- can drive
// the Rust translation exactly the way it drives the C shared object.

mod imp;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// `void printLine(const char * line)`
///
/// The C body is `if (line != NULL) printf("%s\n", line);` -- a NULL pointer
/// prints nothing at all (not even the newline).
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        return;
    }
    imp::print_line_bytes(CStr::from_ptr(line).to_bytes());
}

/// `void printIntLine(int intNumber)` -- `printf("%d\n", intNumber);`
#[no_mangle]
pub extern "C" fn printIntLine(int_number: c_int) {
    imp::print_int_line(int_number as i32);
}

/// `void bad(void)`
#[no_mangle]
pub extern "C" fn bad() {
    imp::bad();
}

/// `void good(void)`
#[no_mangle]
pub extern "C" fn good() {
    imp::good();
}

/// `int main(int argc, char *argv[])` -- `argc`/`argv` are unused by the C
/// implementation, and it always returns 0.
///
/// `cfg(not(test))`: when this crate is compiled as a *test* harness, libtest
/// supplies its own `main` and the linker rejects two entry symbols. The
/// `cdylib` that the differential tests load is built without `cfg(test)`, so it
/// exports `main` exactly like the C shared object does (see SYMBOLS.md).
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    imp::program_main() as c_int
}
