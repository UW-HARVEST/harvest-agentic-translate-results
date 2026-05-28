// Translated from c_src/src/long.c
// Library reproducing byte-identical output of long_exec.

use std::ffi::c_int;
use std::os::raw::c_uint;
use std::sync::Mutex;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: c_int = 2000;

// Global array, equivalent to the C `int array[ARRAY_SIZE];` symbol.
// We use a Mutex<Vec<i32>> to provide interior mutability for the static.
static ARRAY: Mutex<Vec<i32>> = Mutex::new(Vec::new());

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(fmt: *const u8, ...) -> c_int;
}

// Perform expensive arithmetic on each element.
// Mirrors `perform_expensive_operations` from the C source.
fn perform_expensive_operations(array: &mut [i32]) {
    for i in 0..ARRAY_SIZE {
        let mut x: i32 = array[i];
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);
            x ^= x >> 3;
            // x = x - (x << 1);
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7;
            x = (x / 2).wrapping_add(x % 7);
        }
        array[i] = x;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    // srand(seed)
    unsafe {
        srand(seed);
    }

    let mut guard = ARRAY.lock().unwrap();
    if guard.len() != ARRAY_SIZE {
        guard.resize(ARRAY_SIZE, 0);
    }
    let array: &mut [i32] = guard.as_mut_slice();

    // Initialize array with rand() values.
    for i in 0..ARRAY_SIZE {
        array[i] = unsafe { rand() } as i32;
    }

    // Perform expensive operations ITERATIONS times.
    for _ in 0..ITERATIONS {
        perform_expensive_operations(array);
    }

    // XOR-reduce.
    let mut xor_result: i32 = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= array[i];
    }

    // printf("%d\n", xor_result);
    unsafe {
        printf(b"%d\n\0".as_ptr(), xor_result as c_int);
    }
}
