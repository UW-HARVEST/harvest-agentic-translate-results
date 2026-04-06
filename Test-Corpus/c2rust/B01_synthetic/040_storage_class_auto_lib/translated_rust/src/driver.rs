extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_int) {
    let mut y: ::core::ffi::c_int = 2 as ::core::ffi::c_int * x;
    y += 300 as ::core::ffi::c_int;
    printf(b"%d\n\0" as *const u8 as *const ::core::ffi::c_char, y);
}
