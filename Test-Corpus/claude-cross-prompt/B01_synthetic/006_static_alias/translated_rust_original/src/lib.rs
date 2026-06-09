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

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_long;

// Mirror of the C `static int inner = 1;` inside `static_alias`.
// Using a module-level `static mut` because Rust does not support
// function-local mutable static variables with the same semantics
// as C's local statics.
static mut INNER: c_int = 1;

/// Translation of:
/// ```c
/// int* static_alias(int *outer) {
///   static int inner = 1;
///   if (*outer >= inner) {
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
        if *outer >= INNER {
            INNER = INNER.wrapping_add(*outer);
            &raw mut INNER
        } else {
            *outer = (*outer).wrapping_add(INNER);
            outer
        }
    }
}

// FFI declarations needed to reproduce byte-identical output and
// the exact semantics of C's `strtol` and `printf`.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
}

/// Translation of:
/// ```c
/// int main(int argc, char **argv) { ... }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        if argc != 3 {
            // printf("Error: should only be two (integer) arguments!\n");
            printf(b"Error: should only be two (integer) arguments!\n\0".as_ptr() as *const c_char);
            return 1;
        }

        let arg1: *mut c_char = *argv.offset(1);
        let arg2: *mut c_char = *argv.offset(2);

        let mut end: *mut c_char = std::ptr::null_mut();

        // int initial_value = strtol(argv[1], &end, 10);
        let mut initial_value: c_int = strtol(arg1, &mut end as *mut *mut c_char, 10) as c_int;
        if end == arg1 {
            // printf("Error: first argument must be an integer!\n");
            printf(b"Error: first argument must be an integer!\n\0".as_ptr() as *const c_char);
            return 1;
        }

        // int iterations = strtol(argv[2], &end, 10);
        let iterations: c_int = strtol(arg2, &mut end as *mut *mut c_char, 10) as c_int;
        if end == arg2 {
            // printf("Error: second argument must be an integer!\n");
            printf(b"Error: second argument must be an integer!\n\0".as_ptr() as *const c_char);
            return 1;
        }

        // int *running_sum = &initial_value;
        let mut running_sum: *mut c_int = &mut initial_value as *mut c_int;
        let mut i: c_int = 0;
        while i < iterations {
            running_sum = static_alias(running_sum);
            // printf("%d\n", *running_sum);
            printf(b"%d\n\0".as_ptr() as *const c_char, *running_sum);
            i += 1;
        }

        0
    }
}
