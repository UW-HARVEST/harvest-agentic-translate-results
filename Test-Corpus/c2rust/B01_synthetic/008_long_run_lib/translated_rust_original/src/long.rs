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
pub unsafe extern "C" fn perform_expensive_operations() {
    let mut i: size_t = 0 as size_t;
    while i < ARRAY_SIZE as size_t {
        let mut x: ::core::ffi::c_int = array[i as usize];
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < 100 as ::core::ffi::c_int {
            x = x * 3 as ::core::ffi::c_int + 7 as ::core::ffi::c_int;
            x = x ^ x >> 3 as ::core::ffi::c_int;
            x = x - (x << 1 as ::core::ffi::c_int);
            x = x / 2 as ::core::ffi::c_int + x % 7 as ::core::ffi::c_int;
            j += 1;
        }
        array[i as usize] = x;
        i = i.wrapping_add(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn long_exec(mut seed: ::core::ffi::c_uint) {
    srand(seed);
    let mut i: size_t = 0 as size_t;
    while i < ARRAY_SIZE as size_t {
        array[i as usize] = rand();
        i = i.wrapping_add(1);
    }
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < ITERATIONS {
        perform_expensive_operations();
        i_0 += 1;
    }
    let mut xor_result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i_1: size_t = 0 as size_t;
    while i_1 < ARRAY_SIZE as size_t {
        xor_result ^= array[i_1 as usize];
        i_1 = i_1.wrapping_add(1);
    }
    printf(
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        xor_result,
    );
}
