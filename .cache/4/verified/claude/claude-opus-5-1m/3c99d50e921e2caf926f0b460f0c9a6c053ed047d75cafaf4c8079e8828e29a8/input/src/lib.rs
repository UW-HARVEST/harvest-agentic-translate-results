// Rust translation of the C library in c_src/ (StaticLoop).
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

//! Translation of `c_src/src/staticloop.c` / `c_src/include/staticloop.h`.
//!
//! Public ABI (matches `nm -D` of the C shared library exactly):
//!   * `int  static_sum(int update);`
//!   * `void driver(int update);`
//!
//! Output is produced through the C library's `printf` so that stdout
//! buffering, ordering and formatting are byte-identical to the C build.

// The crate/library name mirrors the C target name (`libStaticLoop.so`).
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// C standard library `printf`, used so the emitted bytes and the stdio
    /// buffering behaviour match the original C library exactly.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Function-local `static int sum = 0;` from `static_sum()`.
///
/// In C this is a single mutable object with static storage duration that
/// lives for the whole lifetime of the loaded library and is shared by every
/// call (and, exactly like the C original, is not thread-safe).
static mut SUM: c_int = 0;

/// `"%d\n"` format string for `printf`, NUL terminated as C requires.
const FMT_D_NL: &[u8; 4] = b"%d\n\0";

/// ```c
/// int
/// static_sum(int update) {
///   static int sum = 0;
///   sum += update;
///   return sum;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    // `sum += update` on the shared static. `wrapping_add` reproduces the
    // two's-complement wrap-around that the C code exhibits in practice on
    // signed overflow (which is UB in C, but must not panic here).
    unsafe {
        SUM = SUM.wrapping_add(update);
        SUM
    }
}

/// ```c
/// /*
///   Maintain a running total using a static variable
///  */
/// void
/// driver(int stride) {
///   for (int i = 0; i < 10; i++) {
///     printf("%d\n", static_sum(i * stride));
///   }
///   return;
/// }
/// ```
///
/// Maintain a running total using a static variable.
#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    let mut i: c_int = 0;
    while i < 10 {
        // `i * stride`: wrapping multiply mirrors the C code's behaviour for
        // large strides instead of panicking on overflow.
        let value = static_sum(i.wrapping_mul(stride));
        unsafe {
            printf(FMT_D_NL.as_ptr() as *const c_char, value);
        }
        i += 1;
    }
}
