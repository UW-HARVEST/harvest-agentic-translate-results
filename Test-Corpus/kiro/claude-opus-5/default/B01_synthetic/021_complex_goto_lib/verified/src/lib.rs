// Rust translation of c_src/src/driver.c
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

use std::ffi::{c_char, c_int};

extern "C" {
    /// C `printf` from libc. Used instead of Rust's `print!` so that output is
    /// written through the very same `stdout` FILE stream (and buffering
    /// discipline) as the original C library, keeping byte-for-byte and
    /// interleaving behaviour identical for any caller.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Writes a NUL-terminated literal through libc's `printf`, exactly as the C
/// original does (`printf("loop\n")`, etc.).
#[inline]
fn c_print(s: &[u8]) {
    debug_assert_eq!(*s.last().unwrap(), 0);
    unsafe {
        printf(s.as_ptr() as *const c_char);
    }
}

/// Translation of:
///
/// ```c
/// void driver(int x, int y) {
///     while (x > 0 || y > 0) {
///         printf("loop\n");
///         if (x == 1 && y == 4) {
///             goto label2;
///         }
/// label1:
///         if (x > 0) {
///             printf("x\n");
///             x--;
///         }
/// label2:
///         if (y == 0) {
///             continue;
///         }
///         printf("y\n");
///         y--;
///         if (x < 3) {
///             goto label1;
///         }
///     }
/// }
/// ```
///
/// The unstructured control flow is reproduced with an explicit inner loop:
/// `goto label1` becomes `continue 'body` (re-entering the body *without*
/// re-testing the `while` condition), `continue` becomes `continue 'outer`
/// (re-testing the condition), and falling off the end of the body breaks out
/// of the inner loop back to the condition test. The forward `goto label2`
/// skipping the `label1` block only applies on the first pass of the body for a
/// given outer iteration, so it is tracked by a flag that is cleared once
/// consumed.
///
/// Decrements use wrapping arithmetic: the C code can drive `y` below `INT_MIN`
/// for some inputs (e.g. `x == 1, y == -1` loops forever), which is signed
/// overflow in C. Wrapping matches what the C compiler actually emits rather
/// than panicking.
///
/// # Safety
///
/// This function is `unsafe` only because it is an `extern "C"` export. It
/// dereferences nothing and touches no caller-provided memory, so any `x` and
/// `y` are accepted. The one caveat is inherited from the C: for
/// `x > 0 && y < 0` the loop never terminates, exactly as in
/// `c_src/src/driver.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int, y: c_int) {
    let mut x = x;
    let mut y = y;

    'outer: loop {
        // while (x > 0 || y > 0)
        if !(x > 0 || y > 0) {
            break;
        }

        c_print(b"loop\n\0");

        // if (x == 1 && y == 4) goto label2;
        let mut skip_label1 = x == 1 && y == 4;

        'body: loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    c_print(b"x\n\0");
                    x = x.wrapping_sub(1);
                }
            }
            // The forward jump past `label1` is only taken once, on entry.
            skip_label1 = false;

            // label2:
            if y == 0 {
                continue 'outer;
            }
            c_print(b"y\n\0");
            y = y.wrapping_sub(1);
            if x < 3 {
                // goto label1;
                continue 'body;
            }
            break 'body;
        }
    }
}
