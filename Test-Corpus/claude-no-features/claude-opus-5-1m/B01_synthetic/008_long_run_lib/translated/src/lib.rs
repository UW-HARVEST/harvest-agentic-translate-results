use std::ffi::{c_char, c_int, c_uint};

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: c_int = 2000;

// Global array
static mut ARRAY: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// Perform expensive arithmetic on each element
fn perform_expensive_operations() {
    unsafe {
        for i in 0..ARRAY_SIZE {
            let mut x: c_int = ARRAY[i];
            for _ in 0..100 {
                x = x.wrapping_mul(3).wrapping_add(7);
                x = x ^ (x >> 3);
                x = x.wrapping_sub(x.wrapping_shl(1));
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

        printf(b"%d\n\0".as_ptr() as *const c_char, xor_result);
    }
}
