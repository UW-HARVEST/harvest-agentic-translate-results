// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C translation unit exports four functions with external linkage
//! (`printLine`, `bad`, `good`, `driver`). `driver.h` contains no namespace
//! renaming macros, so the linker symbols are the source-level names verbatim.
//!
//! `bad()` deliberately reads an *uninitialized* automatic pointer variable
//! (CWE-457). That defect is **not fixed** here; see `bad()` for how its
//! observable behaviour is reproduced.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

// Output is emitted through libc's `printf` so that the produced bytes, the
// `%s` conversion semantics, and stdout's buffering/flush ordering are
// identical to the C library, including when this cdylib is loaded next to a C
// caller that also writes to stdout.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%s\n"` format string used by `printLine`.
const FMT_LINE: &[u8; 4] = b"%s\n\0";

/// C: `void printLine(const char *line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    // Order of checks preserved: NULL guard first, then the formatted write.
    if !line.is_null() {
        unsafe {
            printf(FMT_LINE.as_ptr() as *const c_char, line);
        }
    }
}

/// C: `void bad(void)`
///
/// ```c
/// char *data;
/// printLine(data);   // `data` is never assigned -- CWE-457
/// ```
///
/// The bug is preserved, not fixed: `printLine` is still reached with a pointer
/// that the C code never initialized, so the emitted line is empty rather than
/// `"string"`.
///
/// On the *mechanism*: gcc `-O0` lays `data` out at `rsp_at_entry - 16` and
/// loads it unconditionally. Measurements of the compiled C library show that
/// slot consistently holds a leftover pointer to page-aligned memory (observed
/// values `0x…980`, whose target's first byte is `0x00`), so `printLine` sees a
/// non-NULL pointer to an empty string and prints a single `'\n'`.
///
/// That value cannot be reproduced portably: the adjacent stack words hold
/// zeros, small integers and code pointers, so a Rust frame of a different size
/// would read NULL (printing nothing) or a wild pointer (crashing). Reading a
/// genuinely uninitialized slot in Rust was measured to yield NULL, i.e. no
/// output at all. To stay byte-identical, the *observable effect* is reproduced
/// deterministically: `data` points at a NUL byte, exactly as the C program's
/// garbage pointer does.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    /// Stands in for the uninitialized stack slot: non-NULL, first byte `0x00`.
    static UNINITIALIZED_SLOT: [c_char; 1] = [0];

    let data: *const c_char = UNINITIALIZED_SLOT.as_ptr();
    unsafe {
        printLine(data);
    }
}

/// C: `void good(void)`
///
/// ```c
/// char *data;
/// data = "string";
/// printLine(data);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: *const c_char = c"string".as_ptr();
    unsafe {
        printLine(data);
    }
}

/// C: `void driver(int useGood)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() }
    } else {
        unsafe { bad() }
    }
}
