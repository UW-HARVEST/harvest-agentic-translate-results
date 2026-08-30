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

//! Rust translation of `c_src/src/long.c`.
//!
//! Public ABI reproduced (as exported by the C shared library):
//!   * `array`                         — global object, `int[256 * 1024]`
//!   * `perform_expensive_operations`  — `void perform_expensive_operations()`
//!   * `long_exec`                     — `void long_exec(unsigned int seed)`

#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_uint};

// #define ARRAY_SIZE (256 * 1024) // 1MB assuming sizeof(int) = 4
const ARRAY_SIZE: usize = 256 * 1024;
// #define ITERATIONS 2000
const ITERATIONS: c_int = 2000;

extern "C" {
    /// C standard library `srand`, used so the generated pseudo random
    /// sequence — and therefore the program output — matches the C build
    /// byte for byte.
    fn srand(seed: c_uint);
    /// C standard library `rand`.
    fn rand() -> c_int;
    /// C standard library `printf`, used so the output is emitted through the
    /// very same stdio stream (and buffering behaviour) as the C library.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// int array[ARRAY_SIZE];
//
// A zero initialized global lands in `.bss`, exactly like the C definition,
// and is exported from the shared object under the unmangled name `array`.
#[unsafe(no_mangle)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

/// Borrow the global `array` as a slice without going through a reference to
/// a `static mut` (keeps the internals safe-ish while preserving the ABI).
#[inline(always)]
unsafe fn array_mut() -> &'static mut [c_int] {
    std::slice::from_raw_parts_mut((&raw mut array) as *mut c_int, ARRAY_SIZE)
}

/// One trip of the inner `for (int j = 0; j < 100; j++)` loop.
///
/// The C code relies on wrap-around behaviour of `int` arithmetic as produced
/// by the usual two's complement code generation, so every potentially
/// overflowing operation is spelled out with the wrapping variant here:
///
/// ```c
/// x = x * 3 + 7;
/// x = x ^ (x >> 3);
/// x = x - (x << 1);
/// x = x / 2 + x % 7;
/// ```
#[inline(always)]
fn step(mut x: c_int) -> c_int {
    // x = x * 3 + 7;
    x = x.wrapping_mul(3).wrapping_add(7);
    // x = x ^ (x >> 3);  -- arithmetic shift right, as gcc performs it.
    x ^= x >> 3;
    // x = x - (x << 1);
    x = x.wrapping_sub(((x as c_uint) << 1) as c_int);
    // x = x / 2 + x % 7;  -- C truncating division / remainder.
    x = (x / 2).wrapping_add(x % 7);
    x
}

/// The complete inner loop for a single element.
#[inline(always)]
fn churn(mut x: c_int) -> c_int {
    let mut j: c_int = 0;
    while j < 100 {
        x = step(x);
        j += 1;
    }
    x
}

/// How many elements are processed side by side. Each array element is
/// transformed completely independently of every other one, so running the
/// `j` loop for a small batch of neighbours at once yields exactly the same
/// values while letting the code generator emit SIMD instructions — which is
/// what GCC's outer-loop vectoriser does for the original C.
const LANES: usize = 8;

/// ```c
/// void perform_expensive_operations() {
///     for (size_t i = 0; i < ARRAY_SIZE; i++) {
///         int x = array[i];
///         for (int j = 0; j < 100; j++) { ... }
///         array[i] = x;
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_expensive_operations() {
    let a = array_mut();

    let mut chunks = a.chunks_exact_mut(LANES);
    for chunk in &mut chunks {
        let mut v = [0 as c_int; LANES];
        v.copy_from_slice(chunk);
        let mut j: c_int = 0;
        while j < 100 {
            for lane in v.iter_mut() {
                *lane = step(*lane);
            }
            j += 1;
        }
        chunk.copy_from_slice(&v);
    }

    // Tail (never taken for ARRAY_SIZE == 256 * 1024, kept for completeness).
    for slot in chunks.into_remainder().iter_mut() {
        *slot = churn(*slot);
    }
}

/// ```c
/// void long_exec(unsigned int seed) {
///     srand(seed);
///     for (size_t i = 0; i < ARRAY_SIZE; i++) array[i] = rand();
///     for (int i = 0; i < ITERATIONS; i++) perform_expensive_operations();
///     int xor_result = 0;
///     for (size_t i = 0; i < ARRAY_SIZE; i++) xor_result ^= array[i];
///     printf("%d\n", xor_result);
///     return;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn long_exec(seed: c_uint) {
    srand(seed);

    {
        let a = array_mut();
        for slot in a.iter_mut() {
            *slot = rand();
        }
    }

    let mut i: c_int = 0;
    while i < ITERATIONS {
        perform_expensive_operations();
        i += 1;
    }

    let mut xor_result: c_int = 0;
    {
        let a = array_mut();
        for slot in a.iter() {
            xor_result ^= *slot;
        }
    }

    printf(b"%d\n\0".as_ptr() as *const c_char, xor_result);
}
