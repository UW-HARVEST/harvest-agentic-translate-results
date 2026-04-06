extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn printHexCharLine(mut charHex: ::core::ffi::c_char) {
    printf(
        b"%02x\n\0" as *const u8 as *const ::core::ffi::c_char,
        charHex as ::core::ffi::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut data: ::core::ffi::c_char) {
    let mut result: ::core::ffi::c_char =
        (data as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as ::core::ffi::c_char;
    printHexCharLine(result);
}
