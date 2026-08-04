use std::ffi::{c_char, c_int, c_uint};

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: c_int = 2000;

#[unsafe(no_mangle)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    for i in 0..ARRAY_SIZE {
        let mut x = unsafe { array[i] };
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x / 2 + x % 7;
        }
        unsafe {
            array[i] = x;
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
            array[i] = rand();
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result: c_int = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= unsafe { array[i] };
    }

    unsafe {
        printf(c"%d\n".as_ptr(), xor_result);
    }
}
