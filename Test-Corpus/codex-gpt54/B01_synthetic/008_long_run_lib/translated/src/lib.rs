use core::ffi::c_uint;
use libc::{c_int, printf, rand, srand};

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

static mut ARRAY: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

fn perform_expensive_operations() {
    for i in 0..ARRAY_SIZE {
        let mut x = unsafe { ARRAY[i] };
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x << 1);
            x = (x / 2).wrapping_add(x % 7);
        }
        unsafe {
            ARRAY[i] = x;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    unsafe {
        srand(seed);
    }

    for i in 0..ARRAY_SIZE {
        unsafe {
            ARRAY[i] = rand();
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result: c_int = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= unsafe { ARRAY[i] };
    }

    unsafe {
        printf(c"%d\n".as_ptr(), xor_result);
    }
}
