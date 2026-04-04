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
unsafe extern "C" fn helperBad() -> *mut ::core::ffi::c_char {
    let mut charString: [::core::ffi::c_char; 17] =
        ::core::mem::transmute::<[u8; 17], [::core::ffi::c_char; 17]>(*b"helperBad string\0");
    return &raw mut charString as *mut ::core::ffi::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    printLine(helperBad());
}
unsafe extern "C" fn helperGood1() -> *mut ::core::ffi::c_char {
    static mut charString: [::core::ffi::c_char; 19] = unsafe {
        ::core::mem::transmute::<[u8; 19], [::core::ffi::c_char; 19]>(*b"helperGood1 string\0")
    };
    return &raw mut charString as *mut ::core::ffi::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    printLine(helperGood1());
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: ::core::ffi::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
}
