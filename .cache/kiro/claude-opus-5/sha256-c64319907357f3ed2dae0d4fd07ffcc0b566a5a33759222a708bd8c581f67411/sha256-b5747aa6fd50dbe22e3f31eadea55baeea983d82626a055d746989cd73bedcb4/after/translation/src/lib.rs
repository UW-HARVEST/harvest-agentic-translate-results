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
//! The C original uses `goto` to jump both forward (`label2`) and backward
//! (`label1`) inside the body of a `while` loop. The translation below models
//! those jumps with an explicit entry point for each iteration of an inner
//! loop, which reproduces the original control flow exactly.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    // Use C's `printf` so that stdout buffering / flushing behavior (and hence
    // the exact byte stream, including interleaving with any other C output)
    // matches the original library.
    unsafe fn printf(format: *const c_char, ...) -> c_int;
}

/// Writes a NUL-terminated byte literal through C `printf`.
fn print_c(bytes: &'static [u8]) {
    debug_assert_eq!(bytes.last(), Some(&0));
    unsafe {
        printf(bytes.as_ptr() as *const c_char);
    }
}

/// Where the inner (post-`printf("loop\n")`) portion of the loop body starts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// `label1:` — the `if (x > 0)` block.
    Label1,
    /// `label2:` — the `if (y == 0)` block, skipping the `x` block.
    Label2,
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let mut x = x;
    let mut y = y;

    'while_loop: while x > 0 || y > 0 {
        print_c(b"loop\n\0");

        // if (x == 1 && y == 4) goto label2;
        let mut entry = if x == 1 && y == 4 {
            Entry::Label2
        } else {
            Entry::Label1
        };

        loop {
            if entry == Entry::Label1 {
                // label1:
                if x > 0 {
                    print_c(b"x\n\0");
                    x = x.wrapping_sub(1);
                }
            }

            // label2:
            if y == 0 {
                // continue; -> re-evaluate the while condition
                continue 'while_loop;
            }
            print_c(b"y\n\0");
            y = y.wrapping_sub(1);
            if x < 3 {
                // goto label1;
                entry = Entry::Label1;
                continue;
            }

            // Fall off the end of the loop body.
            break;
        }
    }
}
