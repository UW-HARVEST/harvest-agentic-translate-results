extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const ::core::ffi::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const ::core::ffi::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    printLine(data);
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut data: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    data = b"string\0" as *const u8 as *const ::core::ffi::c_char as *mut ::core::ffi::c_char;
    printLine(data);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: ::core::ffi::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
}
