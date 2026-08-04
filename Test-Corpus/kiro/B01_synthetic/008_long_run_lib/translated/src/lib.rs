use std::ffi::c_int;
use std::ffi::c_uint;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;

static mut ARRAY: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

unsafe fn perform_expensive_operations() {
    for i in 0..ARRAY_SIZE {
        let mut x = unsafe { ARRAY[i] };
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x = x ^ (x >> 3);
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x / 2 + x % 7;
        }
        unsafe { ARRAY[i] = x };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    unsafe {
        libc::srand(seed);
        for i in 0..ARRAY_SIZE {
            ARRAY[i] = libc::rand();
        }
        for _ in 0..ITERATIONS {
            perform_expensive_operations();
        }
        let mut xor_result: c_int = 0;
        for i in 0..ARRAY_SIZE {
            xor_result ^= ARRAY[i];
        }
        libc::printf(c"%d\n".as_ptr(), xor_result);
    }
}
