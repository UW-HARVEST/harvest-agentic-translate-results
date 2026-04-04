extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const ::core::ffi::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const ::core::ffi::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut data: ::core::ffi::c_int) {
    let mut source: [::core::ffi::c_char; 100] = [0; 100];
    let mut dest: [::core::ffi::c_char; 100] = ::core::mem::transmute::<
        [u8; 100],
        [::core::ffi::c_char; 100],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    memset(
        &raw mut source as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        'A' as i32,
        (100 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as size_t,
    );
    source[(100 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as usize] =
        '\0' as i32 as ::core::ffi::c_char;
    if data < 100 as ::core::ffi::c_int {
        strncpy(
            &raw mut dest as *mut ::core::ffi::c_char,
            &raw mut source as *mut ::core::ffi::c_char,
            data as size_t,
        );
        dest[data as usize] = '\0' as i32 as ::core::ffi::c_char;
    }
    printLine(&raw mut dest as *mut ::core::ffi::c_char);
}
