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
//
//! C-ABI surface of the translation of `c_src/src/main.c`.
//!
//! Every symbol that the C translation unit exports when it is compiled as a
//! shared object (`printLine`, `bad`, `good`, `main`) is re-exported here with
//! the exact same name and signature. The `static` C helpers (`helperBad`,
//! `helperGood`) are intentionally *not* exported, matching the C `.so`.

use std::ffi::CStr;
use std::os::raw::c_char;

mod imp;

/// `void printLine(const char *line)`
///
/// A NULL pointer prints nothing (the C `if (line != NULL)` guard); otherwise
/// the NUL-terminated bytes are printed verbatim followed by `\n`.
///
/// # Safety
/// `line` must be NULL or a pointer to a NUL-terminated byte string.
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        imp::print_line(None);
        return;
    }
    imp::print_line(Some(CStr::from_ptr(line).to_bytes()));
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

/// `int main(int argc, char *argv[])`
///
/// Exported for symbol parity with the C shared object, which also exports
/// `main`. `argc`/`argv` are ignored, exactly as in the C source, and the
/// function returns 0 without terminating the process.
///
/// `cfg(not(test))` only excludes it from the crate's own unit-test binary,
/// whose Rust-generated entry point would otherwise clash with this symbol;
/// the `cdylib` (and therefore every differential test, which loads the `.so`
/// through `dlopen`) always contains it.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(
    _argc: std::os::raw::c_int,
    _argv: *mut *mut c_char,
) -> std::os::raw::c_int {
    imp::c_main() as std::os::raw::c_int
}
