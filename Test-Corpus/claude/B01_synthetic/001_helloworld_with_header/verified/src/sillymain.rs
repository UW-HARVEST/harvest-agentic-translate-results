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

// Translation of c_src/src/sillymain.c (declared in c_src/src/sillymain.h).

use std::os::raw::{c_char, c_int};

extern "C" {
    /// `<stdio.h>`'s `printf`, i.e. the very function the C source calls.
    ///
    /// The translation deliberately goes through the platform C library instead
    /// of `std::io::stdout()` so that the observable behaviour is identical to
    /// the C original in *every* respect, not just in the bytes eventually
    /// produced:
    ///
    /// * the same `FILE *stdout` object and therefore the same buffering mode
    ///   (unbuffered / line buffered on a tty, fully buffered on a pipe or
    ///   file, as decided by glibc on first use);
    ///   `std::io::stdout()` is always line buffered and would flush at
    ///   different times, which is observable by any other writer of fd 1 and
    ///   by anyone reading the pipe before the process exits;
    /// * the same flush-at-`exit()` semantics (and the same loss of buffered
    ///   output if the process is killed rather than exiting normally);
    /// * the same interleaving with output produced by other C stdio users in
    ///   the process.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// The format string of the `printf` call in `sillymain.c`, NUL terminated
/// exactly as the C string literal is.
const HELLO_WORLD_FORMAT: &[u8] = b"Hello World!\n\0";

/// Mirrors `int helloworld()` from sillymain.c:
///
/// ```c
/// int helloworld() {
///     printf("Hello World!\n");
///     return 0;
/// }
/// ```
pub fn helloworld() -> i32 {
    // SAFETY: `HELLO_WORLD_FORMAT` is a NUL-terminated string literal that
    // contains no conversion specifications, so `printf` consumes no variadic
    // arguments and reads nothing past the terminator.
    let _ = unsafe { printf(HELLO_WORLD_FORMAT.as_ptr() as *const c_char) };
    // C discards printf's result: a write failure (EBADF, ENOSPC, EPIPE, ...)
    // does not change the value returned by helloworld().
    0
}

