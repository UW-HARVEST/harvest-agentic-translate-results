extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strcspn(
        __s: *const ::core::ffi::c_char,
        __reject: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_ulong;
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut s1: *const ::core::ffi::c_char,
    mut s2: *const ::core::ffi::c_char,
) {
    printf(
        b"%zu\n\0" as *const u8 as *const ::core::ffi::c_char,
        strcspn(s1, s2),
    );
}
