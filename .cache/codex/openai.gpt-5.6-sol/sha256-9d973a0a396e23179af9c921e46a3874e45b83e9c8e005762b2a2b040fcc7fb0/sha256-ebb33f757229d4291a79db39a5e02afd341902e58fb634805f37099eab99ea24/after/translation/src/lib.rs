use std::ffi::{c_char, c_int, c_uint};
use std::ptr;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    let base = ptr::addr_of_mut!(array).cast::<c_int>();

    for i in 0..ARRAY_SIZE {
        // C signed overflow is undefined, but the reference build performs
        // these operations as wrapping two's-complement arithmetic.
        let mut x = unsafe { base.add(i).read() };
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x / 2 + x % 7;
        }
        unsafe { base.add(i).write(x) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    let base = ptr::addr_of_mut!(array).cast::<c_int>();

    unsafe { srand(seed) };
    for i in 0..ARRAY_SIZE {
        unsafe { base.add(i).write(rand()) };
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= unsafe { base.add(i).read() };
    }

    unsafe {
        printf(c"%d\n".as_ptr(), xor_result);
    }
}
