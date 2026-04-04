extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn helloworld() -> ::core::ffi::c_int {
    printf(b"Hello World!\n\0" as *const u8 as *const ::core::ffi::c_char);
    return 0 as ::core::ffi::c_int;
}
