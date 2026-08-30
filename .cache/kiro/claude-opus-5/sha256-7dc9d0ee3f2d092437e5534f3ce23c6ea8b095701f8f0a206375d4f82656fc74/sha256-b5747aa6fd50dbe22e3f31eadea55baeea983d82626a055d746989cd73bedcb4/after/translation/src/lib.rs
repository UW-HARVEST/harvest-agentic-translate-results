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
//! Fidelity notes:
//! * `rand()` / `srand()` are taken from the platform C library rather than
//!   reimplemented, so the pseudo-random stream (and therefore the final XOR
//!   value) is bit-for-bit identical to the original program.
//! * The inner arithmetic in `perform_expensive_operations` overflows signed
//!   `int` constantly. C compilers in practice emit plain two's-complement
//!   wrapping instructions for this, so the wrapping operators are used here to
//!   reproduce the observed behaviour exactly (including the arithmetic right
//!   shift of negative values and truncating-toward-zero `/` and `%`).
//! * Output goes through the C library's `printf` so buffering and flushing
//!   match the original translation unit byte for byte.

use std::ffi::{c_char, c_int, c_uint};

/// `#define ARRAY_SIZE (256 * 1024)` — 1MB assuming sizeof(int) == 4.
const ARRAY_SIZE: usize = 256 * 1024;

/// `#define ITERATIONS 2000`
const ITERATIONS: c_int = 2000;

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

// Global array (zero initialised, lives in .bss just like the C original).
#[unsafe(export_name = "array")]
pub static mut ARRAY: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

/// Borrow the global array as a slice so the hot loops can be written in safe
/// Rust. Taking the address of the `static mut` avoids creating overlapping
/// references at any point.
#[inline]
fn array_mut() -> &'static mut [c_int; ARRAY_SIZE] {
    unsafe { &mut *std::ptr::addr_of_mut!(ARRAY) }
}

/// Perform expensive arithmetic on each element.
#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    let array = array_mut();
    for i in 0..ARRAY_SIZE {
        let mut x: c_int = array[i];
        for _j in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x / 2 + x % 7;
        }
        array[i] = x;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    unsafe { srand(seed) };

    {
        let array = array_mut();
        for i in 0..ARRAY_SIZE {
            array[i] = unsafe { rand() };
        }
    }

    for _i in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result: c_int = 0;
    {
        let array = array_mut();
        for i in 0..ARRAY_SIZE {
            xor_result ^= array[i];
        }
    }

    unsafe { printf(c"%d\n".as_ptr(), xor_result) };
}
