// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;
use std::ffi::c_uint;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: c_int = 2000;

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(fmt: *const u8, ...) -> c_int;
}

// Global array. Use a static mut to mirror the C global.
static mut ARRAY: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

fn perform_expensive_operations() {
    unsafe {
        for i in 0..ARRAY_SIZE {
            let mut x: c_int = ARRAY[i];
            for _ in 0..100 {
                // x = x * 3 + 7
                x = x.wrapping_mul(3).wrapping_add(7);
                // x = x ^ (x >> 3)
                x ^= x >> 3;
                // x = x - (x << 1)
                x = x.wrapping_sub(x.wrapping_shl(1));
                // x = x / 2 + x % 7
                x = (x / 2).wrapping_add(x % 7);
            }
            ARRAY[i] = x;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    unsafe {
        srand(seed);

        for i in 0..ARRAY_SIZE {
            ARRAY[i] = rand();
        }

        for _ in 0..ITERATIONS {
            perform_expensive_operations();
        }

        let mut xor_result: c_int = 0;
        for i in 0..ARRAY_SIZE {
            xor_result ^= ARRAY[i];
        }

        printf(b"%d\n\0".as_ptr(), xor_result);
    }
}
