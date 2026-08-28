// Rust translation of c_src/src/staticloop.c
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

// The crate is named `StaticLoop` so the produced shared object is
// `libStaticLoop.so`, matching the CMake target of the original library.
#![allow(non_snake_case)]

use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

unsafe extern "C" {
    /// Use C's `printf` directly so that output formatting *and* stdio
    /// buffering behaviour are byte-for-byte identical to the original
    /// library (including interleaving with any other C stdio output in the
    /// hosting process).
    fn printf(format: *const std::ffi::c_char, ...) -> c_int;
}

/// The function-local `static int sum = 0;` from `static_sum`.
///
/// The C original is a plain, non-atomic `int` with static storage duration
/// that is zero-initialised once for the lifetime of the process. `AtomicI32`
/// gives the same single-instance, process-wide lifetime while letting the
/// Rust code stay free of `static mut`.
static SUM: AtomicI32 = AtomicI32::new(0);

/// ```c
/// int static_sum(int update);
/// ```
///
/// Adds `update` to the running total and returns the new total.
#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    // `sum += update;` — C signed overflow is undefined behaviour, but every
    // mainstream C implementation wraps on two's-complement hardware, which is
    // what the original library does in practice. `wrapping_add` reproduces
    // that observable behaviour instead of panicking.
    let sum = SUM
        .load(Ordering::Relaxed)
        .wrapping_add(update);
    SUM.store(sum, Ordering::Relaxed);
    sum
}

/// ```c
/// void driver(int update);   /* declared as `stride` in the definition */
/// ```
///
/// Maintain a running total using a static variable.
#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    let mut i: c_int = 0;
    while i < 10 {
        // printf("%d\n", static_sum(i * stride));
        let value = static_sum(i.wrapping_mul(stride));
        unsafe {
            printf(c"%d\n".as_ptr(), value);
        }
        i += 1;
    }
}
