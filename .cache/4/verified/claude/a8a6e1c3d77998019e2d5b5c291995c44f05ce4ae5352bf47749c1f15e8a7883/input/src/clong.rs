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

//! Translation of `c_src/src/long.c`.

#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_uint};

/// `#define ARRAY_SIZE (256 * 1024) // 1MB assuming sizeof(int) = 4`
const ARRAY_SIZE: usize = 256 * 1024;

/// `#define ITERATIONS 2000`
const ITERATIONS: c_int = 2000;

unsafe extern "C" {
    /// `<stdlib.h>`: `void srand(unsigned int)`
    fn srand(seed: c_uint);
    /// `<stdlib.h>`: `int rand(void)`
    fn rand() -> c_int;
    /// `<stdio.h>`: `int printf(const char *, ...)`
    fn printf(format: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Global array
// ---------------------------------------------------------------------------
//
// `int array[ARRAY_SIZE];` — a tentative definition at file scope with external
// linkage, so it is an exported (`B`, i.e. `.bss`) symbol of the shared object.

#[unsafe(no_mangle)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

/// The body of the innermost `for (int j = 0; j < 100; j++)` loop of
/// `perform_expensive_operations`.
///
/// The C code relies on wrapping signed arithmetic (`x * 3 + 7`, `x << 1`,
/// `x - ...`), which is technically undefined behaviour in C but is compiled by
/// gcc/clang into plain two's-complement wrapping instructions.  The wrapping
/// operators below reproduce exactly what the C library does at runtime.
/// Likewise `x >> 3` is an arithmetic (sign-propagating) shift, and `x / 2` /
/// `x % 7` truncate towards zero — identical semantics in Rust.
#[inline(always)]
fn mix(mut x: c_int) -> c_int {
    let mut j: c_int = 0;
    while j < 100 {
        // x = x * 3 + 7;
        x = x.wrapping_mul(3).wrapping_add(7);
        // x = x ^ (x >> 3);
        x ^= x >> 3;
        // x = x - (x << 1);
        x = x.wrapping_sub(x.wrapping_shl(1));
        // x = x / 2 + x % 7;
        x = (x / 2).wrapping_add(x % 7);
        j += 1;
    }
    x
}

/// `void perform_expensive_operations()`
///
/// Perform expensive arithmetic on each element of the global `array`.
#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    // SAFETY: `array` is a plain global with static storage duration, exactly as
    // in the C original, which is single-threaded with respect to this access.
    let slice: &mut [c_int; ARRAY_SIZE] = unsafe { &mut *(&raw mut array) };

    for i in 0..ARRAY_SIZE {
        slice[i] = mix(slice[i]);
    }
}

/// `void long_exec(unsigned int seed)`
#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    // SAFETY: FFI calls into libc plus access to the global `array`, mirroring
    // the single-threaded C original.
    unsafe {
        srand(seed);

        {
            let slice: &mut [c_int; ARRAY_SIZE] = &mut *(&raw mut array);
            for i in 0..ARRAY_SIZE {
                slice[i] = rand();
            }
        }

        let mut i: c_int = 0;
        while i < ITERATIONS {
            perform_expensive_operations();
            i += 1;
        }

        let mut xor_result: c_int = 0;
        {
            let slice: &[c_int; ARRAY_SIZE] = &*(&raw const array);
            for i in 0..ARRAY_SIZE {
                xor_result ^= slice[i];
            }
        }

        printf(c"%d\n".as_ptr(), xor_result);
    }
}
