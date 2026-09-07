// Rust translation of c_src/src/long.c (MIT Lincoln Laboratory, 2025).
//
// The C library is built by CMake as a single shared object and exports
// exactly three public symbols:
//
//     B array
//     T long_exec
//     T perform_expensive_operations
//
// This crate reproduces that ABI byte-for-byte compatibly, including the
// 1 MiB zero-initialised `array` object living in .bss.
//
// Fidelity notes:
//   * `srand`/`rand` are taken from the host libc, exactly as the C code does,
//     so the pseudo-random fill is identical on any platform the C library
//     would run on.
//   * Output is emitted through libc `printf` with the same `"%d\n"` format so
//     it shares the C stdio buffer and produces identical bytes.
//   * All integer arithmetic uses wrapping semantics, matching what a C
//     compiler emits for `int` on two's-complement hardware (the C code relies
//     on signed overflow and on `>>` being an arithmetic shift).

#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_uint};
use std::ptr::addr_of_mut;

/// `#define ARRAY_SIZE (256 * 1024)` -- 1MB assuming sizeof(int) = 4
const ARRAY_SIZE: usize = 256 * 1024;

/// `#define ITERATIONS 2000`
const ITERATIONS: c_int = 2000;

/// Number of times the inner arithmetic kernel is applied per element per call
/// to `perform_expensive_operations`.
const INNER: u32 = 100;

extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Wrapper that reproduces the 32-byte alignment gcc gives the C array.
#[repr(C, align(32))]
pub struct Array([c_int; ARRAY_SIZE]);

/// `int array[ARRAY_SIZE];` -- global, zero initialised, exported from the
/// shared object as a 0x100000 byte .bss object.
#[used]
#[unsafe(no_mangle)]
pub static mut array: Array = Array([0; ARRAY_SIZE]);

#[inline(always)]
fn array_mut() -> &'static mut [c_int; ARRAY_SIZE] {
    // The C code is equally non-thread-safe about this global.
    unsafe { &mut (*addr_of_mut!(array)).0 }
}

/// One application of the C inner-loop body:
///
/// ```c
/// x = x * 3 + 7;
/// x = x ^ (x >> 3);
/// x = x - (x << 1);
/// x = x / 2 + x % 7;
/// ```
///
/// Signed overflow wraps and `>>` is arithmetic, matching gcc/clang on all
/// supported targets. `x / 2` truncates toward zero and `x % 7` keeps the sign
/// of the dividend, exactly as C requires.
#[inline(always)]
pub(crate) fn kernel_step(x: c_int) -> c_int {
    let mut x = x;
    x = x.wrapping_mul(3).wrapping_add(7);
    x ^= x >> 3;
    x = x.wrapping_sub(x.wrapping_shl(1));
    x = x.wrapping_div(2).wrapping_add(x.wrapping_rem(7));
    x
}

/// `f^n(x)` computed the naive way.
#[inline(always)]
pub(crate) fn kernel_iterate(mut x: c_int, n: u32) -> c_int {
    for _ in 0..n {
        x = kernel_step(x);
    }
    x
}

mod fast;

/// `void perform_expensive_operations()`
///
/// Perform expensive arithmetic on each element.
#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    let arr = array_mut();
    for slot in arr.iter_mut() {
        *slot = kernel_iterate(*slot, INNER);
    }
}

/// `void long_exec(unsigned int seed)`
#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    unsafe { srand(seed) };

    let arr = array_mut();
    for slot in arr.iter_mut() {
        *slot = unsafe { rand() };
    }

    // The C loop calls perform_expensive_operations() ITERATIONS times, i.e.
    // each element is pushed through the kernel ITERATIONS * INNER times.
    // Because the elements are independent, that is exactly f^(ITERATIONS*INNER)
    // applied element-wise; `fast::apply_iterations` computes the identical
    // result via exact function-iteration algebra.
    fast::apply_iterations(arr, ITERATIONS as u32 * INNER);

    let mut xor_result: c_int = 0;
    for slot in arr.iter() {
        xor_result ^= *slot;
    }

    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, xor_result);
    }
}

#[cfg(test)]
mod ctests {
    use super::*;

    /// Reproduce the whole `long_exec` pipeline for a reduced iteration count
    /// and compare against values captured from the compiled C library.
    fn xor_for(seed: c_uint, iterations: u32) -> c_int {
        unsafe { srand(seed) };
        let mut v: Vec<c_int> = (0..ARRAY_SIZE).map(|_| unsafe { rand() }).collect();
        fast::apply_iterations(&mut v, iterations * INNER);
        v.iter().fold(0, |a, &b| a ^ b)
    }

    #[test]
    fn matches_c_reference() {
        // Captured from `cc -O2` builds of c_src/src/long.c with ITERATIONS
        // overridden, and from the unmodified library (ITERATIONS = 2000).
        let cases: &[(c_uint, u32, c_int)] = &[
            (42, 1, 423058358),
            (42, 2, 469637527),
            (42, 7, 386411628),
            (42, 50, 371319155),
            (1, 1, 224064738),
            (1, 7, 211437105),
            (42, 2000, 430392287),
            (1, 2000, 42032659),
            (12345, 2000, 241792833),
            // ITERATIONS = 100 (n = 10000, i.e. the cycle-accelerated path with
            // a different exponent) across a spread of seeds.
            (3, 100, 416853440),
            (9, 100, 345423641),
            (100, 100, 394014857),
            (777, 100, 460764907),
            (31337, 100, 26890822),
            (999983, 100, 226495287),
            (2000000000, 100, 281955704),
            (4000000000, 100, 424160907),
        ];
        for &(seed, it, expect) in cases {
            assert_eq!(xor_for(seed, it), expect, "seed={} iterations={}", seed, it);
        }
    }
}
