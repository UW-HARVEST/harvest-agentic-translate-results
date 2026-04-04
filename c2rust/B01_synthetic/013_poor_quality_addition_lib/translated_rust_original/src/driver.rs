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
pub unsafe extern "C" fn bad() {
    let mut intOne: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut intTwo: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut intSum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    printIntLine(intSum);
    printIntLine(intSum);
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut intOne: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut intTwo: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut intSum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    printIntLine(intSum);
    intSum = intOne + intTwo;
    printIntLine(intSum);
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
