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
pub unsafe extern "C" fn printIntLine(mut intNumber: ::core::ffi::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad(mut data: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0;
    let mut buffer: [::core::ffi::c_int; 10] = [0 as ::core::ffi::c_int; 10];
    if data >= 0 as ::core::ffi::c_int {
        buffer[data as usize] = 1 as ::core::ffi::c_int;
        i = 0 as ::core::ffi::c_int;
        while i < 10 as ::core::ffi::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
unsafe extern "C" fn goodG2B() {
    let mut data: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0;
    let mut buffer: [::core::ffi::c_int; 10] = [0 as ::core::ffi::c_int; 10];
    if data >= 0 as ::core::ffi::c_int {
        buffer[data as usize] = 1 as ::core::ffi::c_int;
        i = 0 as ::core::ffi::c_int;
        while i < 10 as ::core::ffi::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
unsafe extern "C" fn goodB2G(mut data: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0;
    let mut buffer: [::core::ffi::c_int; 10] = [0 as ::core::ffi::c_int; 10];
    if data >= 0 as ::core::ffi::c_int && data < 10 as ::core::ffi::c_int {
        buffer[data as usize] = 1 as ::core::ffi::c_int;
        i = 0 as ::core::ffi::c_int;
        while i < 10 as ::core::ffi::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(
            b"ERROR: Array index is out-of-bounds\0" as *const u8 as *const ::core::ffi::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn good(mut data: ::core::ffi::c_int) {
    goodG2B();
    goodB2G(data);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut goodData: ::core::ffi::c_int, mut badData: ::core::ffi::c_int) {
    printLine(b"Calling good()...\0" as *const u8 as *const ::core::ffi::c_char);
    good(goodData);
    printLine(b"Finished good()\0" as *const u8 as *const ::core::ffi::c_char);
    printLine(b"Calling bad()...\0" as *const u8 as *const ::core::ffi::c_char);
    bad(badData);
    printLine(b"Finished bad()\0" as *const u8 as *const ::core::ffi::c_char);
}
