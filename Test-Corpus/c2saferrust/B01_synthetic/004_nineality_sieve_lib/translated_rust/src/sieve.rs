
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub fn sieve(mut val: i32) {
    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}

