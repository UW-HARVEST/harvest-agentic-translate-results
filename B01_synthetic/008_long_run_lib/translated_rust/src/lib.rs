use std::ffi::c_int;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: c_int = 2000;

static mut ARRAY: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

unsafe fn perform_expensive_operations() {
    for i in 0..ARRAY_SIZE {
        let mut x: c_int = unsafe { ARRAY[i] };
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
pub extern "C" fn long_exec(seed: c_int) {
    unsafe {
        libc::srand(seed as libc::c_uint);
        for i in 0..ARRAY_SIZE {
            ARRAY[i] = libc::rand() as c_int;
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
