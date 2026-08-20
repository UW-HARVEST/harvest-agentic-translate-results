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

//! `#[no_mangle] extern "C"` export wrappers for the translation in
//! [`driver`].
//!
//! Every function with external linkage in `c_src/src/main.c` is re-exported
//! here under its exact C name so that `libdriver.so` presents the same ABI
//! surface as a shared-library build of `main.c`:
//!
//! | C symbol       | wrapper           |
//! |----------------|-------------------|
//! | `printLine`    | [`printLine`]     |
//! | `printIntLine` | [`printIntLine`]  |
//! | `bad`          | [`bad`]           |
//! | `good`         | [`good`]          |
//! | `main`         | [`main`]          |

#![allow(non_snake_case)]

mod driver;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// `void printLine(const char *line)`
///
/// # Safety
/// `line` must either be NULL or point to a NUL-terminated byte string, exactly
/// as required by the C original.
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    // C: `if (line != NULL) printf("%s\n", line);`
    let bytes = if line.is_null() {
        None
    } else {
        Some(CStr::from_ptr(line).to_bytes())
    };
    driver::print_line(bytes);
}

/// `void printIntLine(int intNumber)`
#[no_mangle]
pub extern "C" fn printIntLine(intNumber: c_int) {
    driver::print_int_line(intNumber as i32);
}

/// `void bad(void)`
#[no_mangle]
pub extern "C" fn bad() {
    driver::bad();
}

/// `void good(void)`
#[no_mangle]
pub extern "C" fn good() {
    driver::good();
}

/// `int main(int argc, char *argv[])`
///
/// The C `main` ignores both parameters and returns 0.  Exported so that the
/// shared library's symbol table matches a `-shared` build of `main.c`, which
/// also exports `main`.
///
/// # Safety
/// `argc` / `argv` are never dereferenced (neither are they in C), so any values
/// are accepted.
// `cfg(not(test))`: under `--test` rustc synthesises its own `main` entry point,
// which would collide with this export.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    driver::program_main() as c_int
}
