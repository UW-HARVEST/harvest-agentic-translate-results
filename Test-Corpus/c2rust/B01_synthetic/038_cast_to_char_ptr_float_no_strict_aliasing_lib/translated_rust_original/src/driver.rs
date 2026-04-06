extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
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
    let mut raw: [::core::ffi::c_char; 4] = [0; 4];
    memcpy(
        &raw mut raw as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        &raw mut x as *const ::core::ffi::c_void,
        ::core::mem::size_of::<::core::ffi::c_float>() as size_t,
    );
    print_hex(
        &raw mut raw as *mut ::core::ffi::c_char as *mut ::core::ffi::c_uchar,
        ::core::mem::size_of::<[::core::ffi::c_char; 4]>() as ::core::ffi::c_int,
    );
}
