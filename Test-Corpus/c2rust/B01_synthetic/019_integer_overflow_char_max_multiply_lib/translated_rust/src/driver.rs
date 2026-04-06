extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const CHAR_MAX: ::core::ffi::c_int = __SCHAR_MAX__;
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const ::core::ffi::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const ::core::ffi::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn printHexCharLine(mut charHex: ::core::ffi::c_char) {
    printf(
        b"%02x\n\0" as *const u8 as *const ::core::ffi::c_char,
        charHex as ::core::ffi::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: ::core::ffi::c_char = 0;
    data = CHAR_MAX as ::core::ffi::c_char;
    if data as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        let mut result: ::core::ffi::c_char =
            (data as ::core::ffi::c_int * 2 as ::core::ffi::c_int) as ::core::ffi::c_char;
        printHexCharLine(result);
    }
}
unsafe extern "C" fn goodG2B() {
    let mut data: ::core::ffi::c_char = 0;
    data = 2 as ::core::ffi::c_char;
    if data as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        let mut result: ::core::ffi::c_char =
            (data as ::core::ffi::c_int * 2 as ::core::ffi::c_int) as ::core::ffi::c_char;
        printHexCharLine(result);
    }
}
unsafe extern "C" fn goodB2G() {
    let mut data: ::core::ffi::c_char = 0;
    data = ' ' as i32 as ::core::ffi::c_char;
    data = CHAR_MAX as ::core::ffi::c_char;
    if data as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        if (data as ::core::ffi::c_int) < CHAR_MAX / 2 as ::core::ffi::c_int {
            let mut result: ::core::ffi::c_char =
                (data as ::core::ffi::c_int * 2 as ::core::ffi::c_int) as ::core::ffi::c_char;
            printHexCharLine(result);
        } else {
            printLine(
                b"data value is too large to perform arithmetic safely.\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: ::core::ffi::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
}
pub const __SCHAR_MAX__: ::core::ffi::c_int = 127 as ::core::ffi::c_int;
