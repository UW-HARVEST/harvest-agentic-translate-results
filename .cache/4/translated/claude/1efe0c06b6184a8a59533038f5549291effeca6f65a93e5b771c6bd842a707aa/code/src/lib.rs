// Rust translation of c_src/ (MIT Lincoln Laboratory "driver" library).
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

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// C `printf` from libc, used so that stdout buffering / interleaving
    /// behaviour is byte-for-byte identical to the original C library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Write a NUL-terminated byte string through C's `printf` using `"%s"`
/// so that any `%` characters in the payload are never interpreted.
#[inline]
fn c_print(s: &[u8]) {
    debug_assert_eq!(s.last(), Some(&0u8));
    unsafe {
        printf(c"%s".as_ptr(), s.as_ptr() as *const c_char);
    }
}

/// Labels in the original C function body, used to reproduce its `goto`
/// driven control flow exactly.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Label {
    /// Top of the `while` body (the `printf("loop\n")` statement).
    Body,
    /// The `label1:` target.
    Label1,
    /// The `label2:` target.
    Label2,
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
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let mut x: c_int = x;
    let mut y: c_int = y;

    // `while (x > 0 || y > 0)`
    while x > 0 || y > 0 {
        // Inner dispatch loop reproducing the backwards `goto label1` and the
        // forwards `goto label2` inside the loop body. Breaking out of this
        // inner loop corresponds to reaching the end of the C loop body (or a
        // `continue`), i.e. re-evaluating the `while` condition.
        let mut label = Label::Body;
        loop {
            match label {
                Label::Body => {
                    c_print(b"loop\n\0");

                    if x == 1 && y == 4 {
                        // goto label2;
                        label = Label::Label2;
                        continue;
                    }

                    // fall through to label1:
                    label = Label::Label1;
                }
                Label::Label1 => {
                    if x > 0 {
                        c_print(b"x\n\0");
                        x = x.wrapping_sub(1);
                    }

                    // fall through to label2:
                    label = Label::Label2;
                }
                Label::Label2 => {
                    if y == 0 {
                        // continue; -> re-test the while condition
                        break;
                    }

                    c_print(b"y\n\0");
                    y = y.wrapping_sub(1);

                    if x < 3 {
                        // goto label1;
                        label = Label::Label1;
                        continue;
                    }

                    // end of the while body -> re-test the while condition
                    break;
                }
            }
        }
    }
}
