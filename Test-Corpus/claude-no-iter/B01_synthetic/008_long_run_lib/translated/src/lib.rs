// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Rust translation of c_src/src/long.c — produces byte-identical output.

use std::ffi::c_int;
use std::ffi::c_uint;
use std::sync::Mutex;
use std::sync::OnceLock;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: usize = 2000;

// Mirror of `int array[ARRAY_SIZE];` global. Stored on the heap to avoid
// blowing the default stack and wrapped in a Mutex to allow safe interior
// mutability of a process-wide global from extern "C" entry points.
fn array_storage() -> &'static Mutex<Vec<c_int>> {
    static ARRAY: OnceLock<Mutex<Vec<c_int>>> = OnceLock::new();
    ARRAY.get_or_init(|| Mutex::new(vec![0 as c_int; ARRAY_SIZE]))
}

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(fmt: *const u8, ...) -> c_int;
}

/// Perform expensive arithmetic on each element.
fn perform_expensive_operations(array: &mut [c_int]) {
    for i in 0..ARRAY_SIZE {
        let mut x: c_int = array[i];
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);  (arithmetic shift on signed int — same as
            // Rust's `>>` for i32, matching GCC's behavior)
            x ^= x >> 3;
            // x = x - (x << 1);  (left shift on signed int — bitwise shift,
            // matching GCC's behavior; equivalent to wrapping_sub of 2*x)
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7;
            x = (x / 2).wrapping_add(x % 7);
        }
        array[i] = x;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    let mut guard = array_storage().lock().expect("array mutex poisoned");
    let array: &mut Vec<c_int> = &mut guard;

    // srand(seed);
    unsafe { srand(seed) };

    // for (size_t i = 0; i < ARRAY_SIZE; i++) array[i] = rand();
    for i in 0..ARRAY_SIZE {
        array[i] = unsafe { rand() };
    }

    // for (int i = 0; i < ITERATIONS; i++) perform_expensive_operations();
    for _ in 0..ITERATIONS {
        perform_expensive_operations(array.as_mut_slice());
    }

    // int xor_result = 0;
    // for (size_t i = 0; i < ARRAY_SIZE; i++) xor_result ^= array[i];
    let mut xor_result: c_int = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= array[i];
    }

    // printf("%d\n", xor_result);
    unsafe {
        printf(b"%d\n\0".as_ptr(), xor_result);
    }
}
