extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn puts(__s: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_int, mut y: ::core::ffi::c_int) {
    let mut result: ::core::ffi::c_int = x | !y;
    printf(b"%d\0" as *const u8 as *const ::core::ffi::c_char, result);
    puts(b"\0" as *const u8 as *const ::core::ffi::c_char);
}
