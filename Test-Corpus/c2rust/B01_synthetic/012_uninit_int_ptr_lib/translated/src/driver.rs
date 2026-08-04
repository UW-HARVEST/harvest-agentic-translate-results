extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn printIntPtrLine(mut intNumber: *const ::core::ffi::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    printIntPtrLine(data);
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut data: ::core::ffi::c_int = 0;
    data = 5 as ::core::ffi::c_int;
    let mut data_addr: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    data_addr = &raw mut data;
    printIntPtrLine(data_addr);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: ::core::ffi::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
}
