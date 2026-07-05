
use std::ffi::CStr;

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
}
#[no_mangle]
pub fn foo(in_0: *const ::core::ffi::c_char, c: ::core::ffi::c_char) -> ::core::ffi::c_int {
    let s = unsafe { CStr::from_ptr(in_0) };
    let target = c as u8;
    s.to_bytes().iter().filter(|&&b| b == target).count() as ::core::ffi::c_int
}

#[no_mangle]
pub fn driver(in_0: &CStr) {
    println!("A: {}", foo(in_0.as_ptr(), b'A' as ::core::ffi::c_char));
    println!("x: {}", foo(in_0.as_ptr(), b'x' as ::core::ffi::c_char));
}

