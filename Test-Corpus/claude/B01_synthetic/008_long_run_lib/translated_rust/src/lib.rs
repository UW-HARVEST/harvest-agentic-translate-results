// Translated from c_src/src/long.c
// Library reproducing byte-identical output of long_exec.

use std::ffi::c_int;
use std::os::raw::c_uint;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: c_int = 2000;

// Global array, equivalent to the C `int array[ARRAY_SIZE];` symbol.
// Exported as `array` so the symbol name matches the C library.
#[unsafe(no_mangle)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(fmt: *const u8, ...) -> c_int;
}

// Perform expensive arithmetic on each element.
// Mirrors `perform_expensive_operations` from the C source.
#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    let arr_ptr = &raw mut array as *mut c_int;
    for i in 0..ARRAY_SIZE {
        // SAFETY: arr_ptr points to a static array of length ARRAY_SIZE.
        let mut x: c_int = unsafe { *arr_ptr.add(i) };
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
        unsafe { *arr_ptr.add(i) = x };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    // srand(seed)
    unsafe {
        srand(seed);
    }

    let arr_ptr = &raw mut array as *mut c_int;

    // Initialize array with rand() values.
    for i in 0..ARRAY_SIZE {
        unsafe { *arr_ptr.add(i) = rand() };
    }

    // Perform expensive operations ITERATIONS times.
    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    // XOR-reduce.
    let mut xor_result: c_int = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= unsafe { *arr_ptr.add(i) };
    }

    // printf("%d\n", xor_result);
    unsafe {
        printf(b"%d\n\0".as_ptr(), xor_result as c_int);
    }
}
