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
//! The C source is:
//!
//! ```c
//! void driver(int x, int y) {
//!     div_t result = div(x, y);
//!     printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//! }
//! ```
//!
//! `driver.h` declares `void driver(int x, int y);` with no namespace-renaming
//! preprocessor macros, so the final linker symbol is plain `driver`.

use std::ffi::c_int;

unsafe extern "C" {
    /// C `printf`. Used instead of Rust's own formatting machinery so that the
    /// bytes written and the stdout buffering behaviour are identical to the C
    /// original (important when a caller mixes its own `printf` calls with
    /// calls into this library).
    fn printf(format: *const std::ffi::c_char, ...) -> c_int;
}

/// Mirror of C's `div_t`.
struct DivT {
    quot: c_int,
    rem: c_int,
}

/// Mirror of C's `div()`.
///
/// `div()` performs truncating division (quotient rounded toward zero) and the
/// remainder has the sign of the numerator; Rust's `/` and `%` on integers use
/// the same rules, so the results agree for every well-defined input.
///
/// `numer / denom` with `denom == 0`, and `c_int::MIN / -1`, are undefined
/// behaviour in C (on x86-64 both raise `SIGFPE`). No attempt is made to
/// "fix" those cases here: Rust's own division aborts the process on them,
/// which is the closest available match, and in particular nothing is written
/// to stdout, just as in the C version.
fn div(numer: c_int, denom: c_int) -> DivT {
    DivT {
        quot: numer / denom,
        rem: numer % denom,
    }
}

/// `void driver(int x, int y)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = div(x, y);
    unsafe {
        printf(
            c"quotient: %d, remainder: %d\n".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
