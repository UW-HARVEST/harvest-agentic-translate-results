// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: usize = 2000;

/// Simple linear congruential generator emulating C's rand()
/// using the POSIX example formula (RAND_MAX = 2^31 - 1).
struct Rand {
    state: u32,
}

impl Rand {
    fn new(seed: u32) -> Self {
        Rand { state: seed }
    }

    fn next(&mut self) -> i32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        (self.state & 0x7FFFFFFF) as i32
    }
}

/// Perform expensive arithmetic on each element of `array`.
fn perform_expensive_operations(array: &mut [i32]) {
    for slot in array.iter_mut() {
        let mut x: i32 = *slot;
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            // Arithmetic shift right by 3 (matches C signed >>)
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            // C's `/` truncates toward zero; Rust's `/` on signed ints does the same.
            // C's `%` follows the sign of the dividend; Rust's `%` matches that.
            x = x / 2 + x % 7;
        }
        *slot = x;
    }
}

#[no_mangle]
pub extern "C" fn long_exec(seed: std::os::raw::c_uint) {
    // Allocate a heap-backed array, mirroring the C global of the same size.
    let mut array: Vec<i32> = vec![0; ARRAY_SIZE];

    let mut rng = Rand::new(seed as u32);
    for slot in array.iter_mut() {
        *slot = rng.next();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &v in array.iter() {
        xor_result ^= v;
    }

    println!("{}", xor_result);
}
