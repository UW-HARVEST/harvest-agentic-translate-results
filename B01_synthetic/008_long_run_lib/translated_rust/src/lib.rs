use std::ffi::c_uint;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;

static mut ARRAY: [i32; ARRAY_SIZE] = [0i32; ARRAY_SIZE];

fn perform_expensive_operations() {
    unsafe {
        for i in 0..ARRAY_SIZE {
            let mut x: i32 = ARRAY[i];
            for _ in 0..100 {
                x = x.wrapping_mul(3).wrapping_add(7);
                x = x ^ (x >> 3);
                x = x.wrapping_sub(x.wrapping_shl(1));
                x = x / 2 + x % 7;
            }
            ARRAY[i] = x;
        }
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

        let mut xor_result: i32 = 0;
        for i in 0..ARRAY_SIZE {
            xor_result ^= ARRAY[i];
        }

        libc::printf(c"%d\n".as_ptr(), xor_result);
    }
}
