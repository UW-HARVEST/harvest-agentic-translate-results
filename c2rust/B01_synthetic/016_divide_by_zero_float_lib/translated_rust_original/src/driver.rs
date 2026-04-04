extern "C" {
    fn fabs(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
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
pub unsafe extern "C" fn printIntLine(mut intNumber: ::core::ffi::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad(mut data: ::core::ffi::c_float) {
    let mut result: ::core::ffi::c_int =
        (100.0f64 / data as ::core::ffi::c_double) as ::core::ffi::c_int;
    printIntLine(result);
}
unsafe extern "C" fn goodG2B() {
    let mut data: ::core::ffi::c_float = 0.;
    data = 2.0f32;
    let mut result: ::core::ffi::c_int =
        (100.0f64 / data as ::core::ffi::c_double) as ::core::ffi::c_int;
    printIntLine(result);
}
unsafe extern "C" fn goodB2G(mut data: ::core::ffi::c_float) {
    if fabs(data as ::core::ffi::c_double) > 0.000001f64 {
        let mut result: ::core::ffi::c_int =
            (100.0f64 / data as ::core::ffi::c_double) as ::core::ffi::c_int;
        printIntLine(result);
    } else {
        printLine(
            b"This would result in a divide by zero\0" as *const u8 as *const ::core::ffi::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn good(mut data: ::core::ffi::c_float) {
    goodG2B();
    goodB2G(data);
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut goodData: ::core::ffi::c_float,
    mut badData: ::core::ffi::c_float,
) {
    printLine(b"Calling good()...\0" as *const u8 as *const ::core::ffi::c_char);
    good(goodData);
    printLine(b"Finished good()\0" as *const u8 as *const ::core::ffi::c_char);
    printLine(b"Calling bad()...\0" as *const u8 as *const ::core::ffi::c_char);
    bad(badData);
    printLine(b"Finished bad()\0" as *const u8 as *const ::core::ffi::c_char);
}
