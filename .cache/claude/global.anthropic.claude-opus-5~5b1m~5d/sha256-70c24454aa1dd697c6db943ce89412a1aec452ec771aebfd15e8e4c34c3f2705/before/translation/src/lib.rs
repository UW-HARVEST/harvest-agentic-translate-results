// Rust translation of the StaticAlias C library.
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

// The crate/library name mirrors the C target name (`StaticAlias`) so that the
// produced shared object is `libStaticAlias.so`, exactly as CMake builds it.
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

// Use the platform C library's `printf` rather than Rust's own buffered
// stdout so that the emitted bytes -- and the buffering/flushing behaviour
// that surrounds them -- match the original C library exactly.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%d\n"` format string used by `driver`, NUL terminated.
static FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// The function-local `static int inner = 1;` of `static_alias`.
///
/// It lives in the shared library's writable data segment exactly like the C
/// original, keeps its value between calls, and its address can be handed back
/// to callers.
static mut INNER: c_int = 1;

/// ```c
/// int *
/// static_alias(int *outer) {
///   static int inner = 1;
///   if(*outer >= inner) {
///     inner += *outer;
///     return &inner;
///   } else {
///     *outer += inner;
///     return outer;
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    unsafe {
        let inner_ptr: *mut c_int = &raw mut INNER;

        // `*outer` is re-read after the potential write to `inner`, because
        // `outer` may alias `inner` itself.
        if *outer >= *inner_ptr {
            // Signed overflow is undefined in C; reproduce the two's
            // complement wrap-around produced in practice.
            *inner_ptr = (*inner_ptr).wrapping_add(*outer);
            inner_ptr
        } else {
            *outer = (*outer).wrapping_add(*inner_ptr);
            outer
        }
    }
}

/// ```c
/// void
/// driver(int initial_value, int iterations) {
///   int *running_sum = &initial_value;
///   for (int i = 0; i < iterations; i++) {
///     running_sum = static_alias(running_sum);
///     printf("%d\n", *running_sum);
///   }
///   return;
/// }
/// ```
///
/// Note that `initial_value` is a by-value parameter, so `running_sum`
/// initially points at the callee's own copy of it, which `static_alias` may
/// mutate through the alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    // Local, mutable copy of the parameter -- the C parameter's stack slot.
    let mut initial_value: c_int = initial_value;

    unsafe {
        let mut running_sum: *mut c_int = &raw mut initial_value;
        let mut i: c_int = 0;
        while i < iterations {
            running_sum = static_alias(running_sum);
            printf(FMT_D_NL.as_ptr(), *running_sum);
            i = i.wrapping_add(1);
        }
    }
}
