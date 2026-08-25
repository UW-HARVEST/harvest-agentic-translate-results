use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn rand() -> c_int;
    fn srand(seed: c_uint);
    static mut stderr: *mut c_void;
    fn strtoul(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong;
}

#[no_mangle]
pub static mut array: [c_int; ARRAY_SIZE] = [0; ARRAY_SIZE];

#[no_mangle]
pub unsafe extern "C" fn perform_expensive_operations() {
    let base = std::ptr::addr_of_mut!(array).cast::<c_int>();

    for index in 0..ARRAY_SIZE {
        let value = base.add(index);
        let mut x = value.read();
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x / 2 + x % 7;
        }
        value.write(x);
    }
}

#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 2 {
        fprintf(stderr, c"Usage: %s <seed>\n".as_ptr(), (*argv).cast_const());
        return 1;
    }

    *__errno_location() = 0;
    let mut endptr = std::ptr::null_mut();
    let argument = *argv.add(1);
    let temporary_seed = strtoul(argument, &mut endptr, 10);
    if *endptr != 0 || *__errno_location() != 0 || temporary_seed > c_uint::MAX as c_ulong {
        fprintf(
            stderr,
            c"Invalid seed: '%s'\n".as_ptr(),
            argument.cast_const(),
        );
        return 1;
    }

    srand(temporary_seed as c_uint);
    let base = std::ptr::addr_of_mut!(array).cast::<c_int>();
    for index in 0..ARRAY_SIZE {
        base.add(index).write(rand());
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result = 0;
    for index in 0..ARRAY_SIZE {
        xor_result ^= base.add(index).read();
    }

    printf(c"%d\n".as_ptr(), xor_result);
    0
}
