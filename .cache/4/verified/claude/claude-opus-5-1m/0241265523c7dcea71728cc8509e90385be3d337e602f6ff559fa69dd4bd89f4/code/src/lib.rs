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

//! C-ABI surface of the translated `driver` program.
//!
//! `c_src/src/main.c` compiles (with `-shared -fPIC`) to a shared object that
//! exports exactly four global symbols:
//!
//! ```text
//! T bad
//! T good
//! T main
//! T printLine
//! ```
//!
//! This crate's `cdylib` mirrors that surface one-for-one so that a caller
//! that `dlopen()`s either object sees the same names with the same
//! signatures and the same observable behavior.  `helperBad` and
//! `helperGood1` are `static` in C (internal linkage) and are therefore
//! deliberately *not* exported here either.

#[path = "prog.rs"]
pub mod prog;

use std::ffi::CStr;
use std::os::raw::c_char;
#[cfg(not(test))]
use std::os::raw::c_int;

/// `void printLine(const char *line)`
///
/// # Safety
/// `line` must either be null or point to a NUL-terminated byte string, which
/// is exactly the contract the C function imposes on its caller.
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    // `if (line != NULL)` — a null pointer produces no output whatsoever.
    if line.is_null() {
        return;
    }
    let bytes = CStr::from_ptr(line).to_bytes();
    prog::print_line(Some(bytes));
    prog::flush_stdout();
}

/// `void bad()`
#[no_mangle]
pub extern "C" fn bad() {
    prog::bad();
    prog::flush_stdout();
}

/// `void good()`
#[no_mangle]
pub extern "C" fn good() {
    prog::good();
    prog::flush_stdout();
}

/// `int main()`
///
/// Suppressed under `cfg(test)` only because libtest generates its own `main`
/// entry symbol; the real `cdylib` is never built with `cfg(test)`, so the
/// export is always present in the shipped shared object.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let rc = prog::c_main();
    prog::flush_stdout();
    rc as c_int
}
