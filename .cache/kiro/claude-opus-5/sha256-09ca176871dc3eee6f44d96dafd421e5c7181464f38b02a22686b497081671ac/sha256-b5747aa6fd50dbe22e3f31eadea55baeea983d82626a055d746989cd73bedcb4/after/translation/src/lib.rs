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

//! Rust translation of `c_src/src/driver.c`.
//!
//! Output is emitted through C `printf` so that stream buffering and
//! interleaving with any C-side output stay byte-for-byte identical to the
//! original library.

use std::ffi::c_char;
use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Emit a NUL-terminated literal through C `printf`.
///
/// The literal is used as the format string, mirroring the original C calls
/// (none of them contain conversion specifiers).
fn c_print(msg: &'static str) {
    debug_assert!(msg.ends_with('\0'));
    unsafe {
        printf(msg.as_ptr() as *const c_char);
    }
}

/// `static int y = 123;` from driver.c — file-scope mutable state.
static Y: AtomicI32 = AtomicI32::new(123);

/// Translation of the file-local `multi_stage` helper.
///
/// The C version uses `goto fail`, so every error path prints its specific
/// message followed by "Operation failed", while the success path returns
/// without printing it.
fn multi_stage(x: c_int, z: c_int) -> c_int {
    let result: c_int;

    // Each check mirrors the C order exactly: x, then y, then z.
    if x != 1 {
        c_print("Error: x != 1\n\0");
        result = 1;
    } else if Y.load(Ordering::Relaxed) != 2 {
        c_print("Error: x == 1 but y != 2\n\0");
        result = 2;
    } else if z != 3 {
        c_print("Error: x == 1 and y == 2, but z != 3\n\0");
        result = 3;
    } else {
        c_print("Ok!\n\0");
        return 0; // `result` is still 0 here in the C original.
    }

    // `fail:` label.
    c_print("Operation failed\n\0");
    result
}

/// Translation of `void driver(int x, int local_y, int z)`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    Y.store(local_y, Ordering::Relaxed);
    let result = multi_stage(x, z);
    unsafe {
        printf(c"Result: %d\n".as_ptr(), result);
    }
}
