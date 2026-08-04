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
    printLine(b"bad()\0" as *const u8 as *const ::core::ffi::c_char);
}
unsafe extern "C" fn helperGood() {
    printLine(b"helperGood()\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    printLine(b"good()\0" as *const u8 as *const ::core::ffi::c_char);
    helperGood();
}
#[no_mangle]
pub unsafe extern "C" fn driver() {
    printLine(b"Calling good()...\0" as *const u8 as *const ::core::ffi::c_char);
    good();
    printLine(b"Finished good()\0" as *const u8 as *const ::core::ffi::c_char);
    printLine(b"Calling bad()...\0" as *const u8 as *const ::core::ffi::c_char);
    bad();
    printLine(b"Finished bad()\0" as *const u8 as *const ::core::ffi::c_char);
}
