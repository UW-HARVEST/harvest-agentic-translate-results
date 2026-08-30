// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// Original copyright notice from the C sources:
//
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

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    /// `printf("%s\n", s)` in the C source; the C compiler lowers that exact
    /// call to `puts`, which is what the reference `.so` imports. Going through
    /// libc (rather than Rust's `std::io`) keeps us on the very same `stdout`
    /// FILE stream and buffering discipline as the C library, so output bytes
    /// and their ordering relative to any C caller are preserved.
    fn puts(s: *const c_char) -> c_int;
}

/// C: `void printLine(const char *line)`
///
/// Prints `line` followed by a newline, but only when `line` is non-NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        puts(line);
    }
}

/// C: `static void helperBad()` — internal linkage, never referenced by the
/// original translation unit either. Kept for fidelity with the C source.
unsafe fn helperBad() {
    printLine(c"helperBad()".as_ptr());
}

/// C: `void bad()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    printLine(c"bad()".as_ptr());
}

/// C: `static void helperGood()` — internal linkage, not exported.
unsafe fn helperGood() {
    printLine(c"helperGood()".as_ptr());
}

/// C: `void good()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    printLine(c"good()".as_ptr());
    helperGood();
}

/// C: `void driver()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver() {
    printLine(c"Calling good()...".as_ptr());
    good();
    printLine(c"Finished good()".as_ptr());
    printLine(c"Calling bad()...".as_ptr());
    bad();
    printLine(c"Finished bad()".as_ptr());
}

// `helperBad` mirrors the unused `static` helper in the C source. Reference it
// from a `#[used]` static so the compiler does not warn about dead code, while
// keeping it out of the exported dynamic symbol table (matching the C `.so`).
#[used]
static _KEEP_HELPER_BAD: unsafe fn() = helperBad;
