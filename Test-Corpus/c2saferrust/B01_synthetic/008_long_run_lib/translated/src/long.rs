

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn rand() -> ::core::ffi::c_int;
    fn srand(__seed: ::core::ffi::c_uint);
}
pub type size_t = usize;
pub const ARRAY_SIZE: ::core::ffi::c_int = 256 as ::core::ffi::c_int * 1024 as ::core::ffi::c_int;
pub const ITERATIONS: ::core::ffi::c_int = 2000 as ::core::ffi::c_int;
#[no_mangle]
pub static mut array: [::core::ffi::c_int; 262144] = [0; 262144];
#[no_mangle]
pub fn perform_expensive_operations() {
    unsafe {
        let mut i = 0usize;
        while i < ARRAY_SIZE as usize {
            let mut x = array[i];
            for _ in 0..100 {
                x = x * 3 + 7;
                x ^= x >> 3;
                x -= x << 1;
                x = x / 2 + x % 7;
            }
            array[i] = x;
            i += 1;
        }
    }
}

#[no_mangle]
pub fn long_exec(seed: u32) {
    unsafe {
        srand(seed);

        let mut i = 0usize;
        while i < ARRAY_SIZE as usize {
            array[i] = rand();
            i += 1;
        }

        let mut i_0 = 0;
        while i_0 < ITERATIONS {
            perform_expensive_operations();
            i_0 += 1;
        }

        let mut xor_result = 0;
        let mut i_1 = 0usize;
        while i_1 < ARRAY_SIZE as usize {
            xor_result ^= array[i_1];
            i_1 += 1;
        }

        println!("{}", xor_result);
    }
}

