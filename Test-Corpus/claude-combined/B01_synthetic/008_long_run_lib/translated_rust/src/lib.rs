// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;
use std::ffi::c_uint;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: c_int = 2000;

// FFI bindings to libc's rand/srand/printf so output and PRNG match C exactly.
extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(fmt: *const u8, ...) -> c_int;
}

// Global array, exported as a public C symbol matching the C version.
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    unsafe {
        let arr = &mut *core::ptr::addr_of_mut!(array);
        for i in 0..ARRAY_SIZE {
            let mut x: c_int = arr[i];
            for _j in 0..100 {
                // x = x * 3 + 7;
                x = x.wrapping_mul(3).wrapping_add(7);
                // x = x ^ (x >> 3);
                x ^= x >> 3;
                // x = x - (x << 1);
                x = x.wrapping_sub(x.wrapping_shl(1));
                // x = x / 2 + x % 7;
                x = (x / 2).wrapping_add(x % 7);
            }
            arr[i] = x;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    unsafe {
        srand(seed);

        let arr = &mut *core::ptr::addr_of_mut!(array);

        for i in 0..ARRAY_SIZE {
            arr[i] = rand();
        }

        for _i in 0..ITERATIONS {
            perform_expensive_operations();
        }

        let mut xor_result: c_int = 0;
        for i in 0..ARRAY_SIZE {
            xor_result ^= arr[i];
        }

        // printf("%d\n", xor_result);
        printf(b"%d\n\0".as_ptr(), xor_result);
    }
}
