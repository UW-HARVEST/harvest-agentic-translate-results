// Rust translation of c_src/src/staticalias.c
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

// The crate/library name intentionally mirrors the C target name
// (`libStaticAlias.so`) rather than Rust naming conventions.
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// C `printf` from the platform libc. Used instead of Rust's own `stdout`
    /// so that the emitted bytes and the stream buffering behaviour match the
    /// original C library exactly (important when a C caller also writes to
    /// stdout in the same process).
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `static int inner = 1;` from `static_alias`.
///
/// Function-local `static` storage in C has the lifetime of the whole program,
/// so the value persists across every call to `static_alias` — including calls
/// made from separate `driver` invocations.
static mut INNER: c_int = 1;

/// Translation of:
///
/// ```c
/// int *static_alias(int *outer) {
///   static int inner = 1;
///   if (*outer >= inner) { inner += *outer; return &inner; }
///   else                 { *outer += inner; return outer;  }
/// }
/// ```
///
/// The returned pointer aliases either the static `inner` or the caller's own
/// object, which is the behaviour the original code intentionally exercises.
/// Additions use wrapping arithmetic to mirror the two's-complement wraparound
/// produced by mainstream C compilers (signed overflow is UB in C, so no
/// behaviour is being "fixed" here).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    unsafe {
        let inner: *mut c_int = &raw mut INNER;

        if *outer >= *inner {
            *inner = (*inner).wrapping_add(*outer);
            inner
        } else {
            *outer = (*outer).wrapping_add(*inner);
            outer
        }
    }
}

/// Translation of:
///
/// ```c
/// void driver(int initial_value, int iterations) {
///   int *running_sum = &initial_value;
///   for (int i = 0; i < iterations; i++) {
///     running_sum = static_alias(running_sum);
///     printf("%d\n", *running_sum);
///   }
/// }
/// ```
///
/// `initial_value` is a by-value parameter in C, so `&initial_value` points at
/// the function's own mutable copy; `running_sum` starts out aliasing that copy
/// and may later be redirected at the static `inner`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    unsafe {
        let mut initial_value: c_int = initial_value;
        let mut running_sum: *mut c_int = &raw mut initial_value;

        let mut i: c_int = 0;
        while i < iterations {
            running_sum = static_alias(running_sum);
            printf(c"%d\n".as_ptr(), *running_sum);
            i = i.wrapping_add(1);
        }
    }
}
