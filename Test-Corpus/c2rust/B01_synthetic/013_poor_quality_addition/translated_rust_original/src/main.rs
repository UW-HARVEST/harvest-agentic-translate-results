#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#[allow(unused_imports)]
use ::driver;
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
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    printLine(b"Calling good()...\0" as *const u8 as *const ::core::ffi::c_char);
    good();
    printLine(b"Finished good()\0" as *const u8 as *const ::core::ffi::c_char);
    printLine(b"Calling bad()...\0" as *const u8 as *const ::core::ffi::c_char);
    bad();
    printLine(b"Finished bad()\0" as *const u8 as *const ::core::ffi::c_char);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
