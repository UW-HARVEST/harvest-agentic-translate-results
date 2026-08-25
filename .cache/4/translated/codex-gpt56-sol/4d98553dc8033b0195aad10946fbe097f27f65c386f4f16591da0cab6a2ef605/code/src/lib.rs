use std::ffi::{c_char, c_int, c_uint};

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;
const INNER_ITERATIONS: usize = 100;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn rand() -> c_int;
    fn srand(seed: c_uint);
}

#[unsafe(no_mangle)]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

#[inline]
fn expensive_operations(mut values: [c_int; 4]) -> [c_int; 4] {
    for _ in 0..INNER_ITERATIONS {
        for value in &mut values {
            *value = value.wrapping_mul(3).wrapping_add(7);
            *value ^= *value >> 3;
            *value = value.wrapping_sub(value.wrapping_shl(1));
            *value = *value / 2 + *value % 7;
        }
    }
    values
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_expensive_operations() {
    let base = core::ptr::addr_of_mut!(array).cast::<c_int>();

    for index in (0..ARRAY_SIZE).step_by(4) {
        // SAFETY: the array length is a multiple of four and `index` is in bounds.
        let slot = unsafe { base.add(index).cast::<[c_int; 4]>() };
        unsafe { slot.write(expensive_operations(slot.read())) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn long_exec(seed: c_uint) {
    unsafe { srand(seed) };

    let base = core::ptr::addr_of_mut!(array).cast::<c_int>();
    for index in 0..ARRAY_SIZE {
        // SAFETY: `index` is within the exported array.
        unsafe { base.add(index).write(rand()) };
    }

    for _ in 0..ITERATIONS {
        unsafe { perform_expensive_operations() };
    }

    let mut xor_result = 0;
    for index in 0..ARRAY_SIZE {
        // SAFETY: `index` is within the exported array.
        xor_result ^= unsafe { base.add(index).read() };
    }

    unsafe { printf(c"%d\n".as_ptr(), xor_result) };
}
