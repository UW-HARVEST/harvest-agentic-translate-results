// Rust translation of c_src/src/driver.c
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

use std::ffi::c_int;
use std::io::Write;

/// The three reachable program points inside the original `while` statement:
/// the loop header (condition test), the `label1:` statement, and the
/// `label2:` statement. Modelling them explicitly lets the backwards
/// `goto label1` and the forwards `goto label2` be reproduced exactly,
/// including the fact that `goto label1` re-enters the loop body *without*
/// re-evaluating the `while` condition.
enum Point {
    LoopHeader,
    Label1,
    Label2,
}

/// Writes a NUL-free line to stdout the way `printf("...\n")` does.
/// Rust's stdout handle is line buffered, so each call is flushed at the
/// newline, matching C's line-buffered stdout ordering.
fn put_line(s: &str) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(s.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let mut x = x;
    let mut y = y;

    let mut point = Point::LoopHeader;

    loop {
        match point {
            Point::LoopHeader => {
                // while (x > 0 || y > 0)
                if !(x > 0 || y > 0) {
                    return;
                }

                put_line("loop\n");

                if x == 1 && y == 4 {
                    point = Point::Label2; // goto label2;
                } else {
                    point = Point::Label1; // fall through to label1:
                }
            }

            Point::Label1 => {
                if x > 0 {
                    put_line("x\n");
                    x = x.wrapping_sub(1);
                }
                point = Point::Label2; // fall through to label2:
            }

            Point::Label2 => {
                if y == 0 {
                    point = Point::LoopHeader; // continue;
                    continue;
                }
                put_line("y\n");
                y = y.wrapping_sub(1);
                if x < 3 {
                    point = Point::Label1; // goto label1;
                } else {
                    point = Point::LoopHeader; // end of loop body
                }
            }
        }
    }
}
