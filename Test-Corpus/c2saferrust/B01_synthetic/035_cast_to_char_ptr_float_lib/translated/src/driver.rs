extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
unsafe extern "C" fn print_hex(mut p: *mut ::core::ffi::c_uchar, mut len: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len {
        printf(
            b"%02x\0" as *const u8 as *const ::core::ffi::c_char,
            *p.offset(i as isize) as ::core::ffi::c_int,
        );
        i += 1;
    }
    printf(b"\n\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_float) {
    print_hex(
        &raw mut x as *mut ::core::ffi::c_uchar,
        ::core::mem::size_of::<::core::ffi::c_float>() as ::core::ffi::c_int,
    );
}
