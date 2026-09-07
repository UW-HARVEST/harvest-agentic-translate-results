// Rust translation of the StaticLoop C library (c_src/src/staticloop.c).
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

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int};

extern "C" {
    // C's `printf`, used so that output goes through the very same stdio
    // stream (and buffering discipline) the C library used.  This keeps the
    // emitted bytes -- and their interleaving with any other C stdio output
    // performed by a host program -- identical.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Wrapper giving a process-wide mutable `int`, mirroring a C function-scope
/// `static int`.  Like the C original this performs no synchronisation.
struct CStatic(UnsafeCell<c_int>);

// SAFETY: matches the (absent) thread-safety guarantees of the C `static int`.
unsafe impl Sync for CStatic {}

/// `static int sum = 0;` from `static_sum()`.
static SUM: CStatic = CStatic(UnsafeCell::new(0));

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
    let sum = SUM.0.get();
    // `wrapping_add` reproduces the two's-complement wrap-around that gcc/clang
    // emit for `sum += update` (signed overflow is UB in C, but in practice it
    // simply wraps on the supported targets).
    unsafe {
        *sum = (*sum).wrapping_add(update);
        *sum
    }
}

/// ```c
/// void
/// driver(int stride) {
///   for (int i = 0; i < 10; i++) {
///     printf("%d\n", static_sum(i * stride));
///   }
///   return;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    let mut i: c_int = 0;
    while i < 10 {
        let value = static_sum(i.wrapping_mul(stride));
        unsafe {
            printf(b"%d\n\0".as_ptr() as *const c_char, value);
        }
        i = i.wrapping_add(1);
    }
}
