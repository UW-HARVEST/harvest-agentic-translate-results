
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub fn helloworld() -> i32 {
    println!("Hello World!");
    0
}

