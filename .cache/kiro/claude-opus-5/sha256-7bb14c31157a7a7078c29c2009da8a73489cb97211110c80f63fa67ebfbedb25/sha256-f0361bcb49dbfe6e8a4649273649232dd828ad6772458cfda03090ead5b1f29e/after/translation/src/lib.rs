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

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// The C source uses `printf` from <stdio.h>. Calling libc's `printf`
    /// directly (rather than Rust's `println!`) keeps the write going through
    /// the exact same `stdout` FILE stream and buffering discipline as the C
    /// library, so byte-for-byte output and flush ordering are preserved.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Rust equivalent of C's `div_t`.
#[repr(C)]
#[derive(Copy, Clone)]
struct DivT {
    quot: c_int,
    rem: c_int,
}

/// Equivalent of C's `div()` from <stdlib.h>.
///
/// C's `div` performs plain `numer / denom` and `numer % denom` on `int`,
/// which on x86-64 lowers to a single `idiv` instruction. That means the two
/// non-representable cases -- division by zero, and `INT_MIN / -1` -- raise
/// SIGFPE rather than returning a value. Rust's `/` operator would instead
/// panic (printing a message to stderr and aborting), which is observably
/// different. Emitting `idiv` directly reproduces the original C behavior
/// exactly, including the fatal signal.
#[cfg(target_arch = "x86_64")]
#[inline]
fn c_div(numer: c_int, denom: c_int) -> DivT {
    let quot: c_int;
    let rem: c_int;
    unsafe {
        std::arch::asm!(
            "cdq",
            "idiv {denom:e}",
            denom = in(reg) denom,
            inout("eax") numer => quot,
            out("edx") rem,
            options(pure, nomem, nostack),
        );
    }
    DivT { quot, rem }
}

/// Portable fallback for non-x86-64 targets. `wrapping_div`/`wrapping_rem`
/// avoid Rust's overflow panic for `INT_MIN / -1` (yielding `INT_MIN`, the
/// value the truncated hardware result also produces); division by zero still
/// aborts, matching the C program's fatal termination.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn c_div(numer: c_int, denom: c_int) -> DivT {
    DivT {
        quot: numer.wrapping_div(denom),
        rem: numer.wrapping_rem(denom),
    }
}

/// void driver(int x, int y);
///
/// Translation of:
/// ```c
/// void driver(int x, int y) {
///     div_t result = div(x, y);
///     printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = c_div(x, y);
    unsafe {
        printf(
            c"quotient: %d, remainder: %d\n".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
