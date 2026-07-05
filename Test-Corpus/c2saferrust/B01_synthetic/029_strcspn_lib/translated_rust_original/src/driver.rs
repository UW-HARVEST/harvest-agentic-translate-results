
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strcspn(
        __s: *const ::core::ffi::c_char,
        __reject: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_ulong;
}
#[no_mangle]
pub fn driver(s1: &str, s2: &str) {
    let result = s1
        .chars()
        .take_while(|c| !s2.contains(*c))
        .count();
    println!("{}", result);
}

