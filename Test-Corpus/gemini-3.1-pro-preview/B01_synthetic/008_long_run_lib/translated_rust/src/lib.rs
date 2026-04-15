use std::os::raw::{c_int, c_uint};

extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
}

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

fn perform_expensive_operations(array: &mut [c_int]) {
    for x in array.iter_mut() {
        let mut val = *x;
        for _ in 0..100 {
            val = val.wrapping_mul(3).wrapping_add(7);
            val ^= val >> 3;
            val = val.wrapping_sub(val << 1);
            val = (val / 2) + (val % 7);
        }
        *x = val;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    unsafe {
        srand(seed);
    }

    let mut array: Vec<c_int> = vec![0; ARRAY_SIZE];

    for x in array.iter_mut() {
        *x = unsafe { rand() };
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: c_int = 0;
    for x in array.iter() {
        xor_result ^= *x;
    }

    println!("{}", xor_result);
}
