use std::os::raw::c_uint;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;

static mut ARRAY: [i32; ARRAY_SIZE] = [0; ARRAY_SIZE];

fn perform_expensive_operations() {
    unsafe {
        for i in 0..ARRAY_SIZE {
            let mut x = ARRAY[i];
            for _ in 0..100 {
                x = x * 3 + 7;
                x = x ^ (x >> 3);
                x = x - (x << 1);
                x = x / 2 + x % 7;
            }
            ARRAY[i] = x;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    let mut rng = seed as u32;
    
    unsafe {
        for i in 0..ARRAY_SIZE {
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            ARRAY[i] = (rng & 0x7fff) as i32;
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result: i32 = 0;
    unsafe {
        for i in 0..ARRAY_SIZE {
            xor_result ^= ARRAY[i];
        }
    }

    println!("{}", xor_result);
}