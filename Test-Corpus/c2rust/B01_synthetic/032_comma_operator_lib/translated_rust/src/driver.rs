extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < x {
        printf(
            b"%d %d\n\0" as *const u8 as *const ::core::ffi::c_char,
            i,
            j,
        );
        i += 1;
        j += 2 as ::core::ffi::c_int;
    }
}
