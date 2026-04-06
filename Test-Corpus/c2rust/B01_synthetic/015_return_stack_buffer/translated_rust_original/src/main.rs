#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
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
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut x,
    );
    if x != 0 {
        good();
    } else {
        bad();
    }
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
