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
/// the C code never initialized, so the emitted line is not `"string"`.
///
/// On the *mechanism*, measured against the compiled C library (gcc -O0,
/// x86_64): `bad` allocates a 16-byte frame and loads `data` from `[rbp-0x8]`
/// without ever storing to it. Nothing in `bad` or `driver` writes that slot, so
/// the value is whatever the previous user of those stack bytes left behind --
/// and because `printLine` and `good` have the *same* frame geometry, `[rbp-0x8]`
/// is exactly where they save their own `char *` when called at the same depth.
/// The C behaviour is therefore decided by frame aliasing, not by the program:
///
/// | preceding call sequence              | measured C output              |
/// |--------------------------------------|--------------------------------|
/// | nothing (`bad` first in the process) | `"\n"` (slot points at a NUL)  |
/// | `driver(0)` first                    | `""` (slot is zero, so NULL)   |
/// | `printLine("MARKER"); bad()`         | `"MARKER"` printed twice       |
/// | `good(); driver(0)`                  | `"string"` printed twice       |
/// | `printLine("MARKER"); driver(0)`     | raw machine-code bytes         |
/// | `driver(1); bad()`                   | SIGSEGV                        |
/// | `driver(0)`/`driver(1)` alternating  | varies between runs            |
///
/// No value this function could pick matches every context, and reproducing the
/// aliasing would mean re-emitting all four functions with gcc's exact frame
/// layout in assembly. So the *observable effect* is reproduced deterministically
/// instead: `data` points at a NUL byte, which is what the C library does
/// whenever `bad()` is the first thing a process calls, and which yields a single
/// `'\n'` -- non-empty, but not the `"string"` that `good()` prints.
///
/// See `translation/tests/external_caller.rs::ub_path_characterisation`, which
/// re-measures the table above on every test run.
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
