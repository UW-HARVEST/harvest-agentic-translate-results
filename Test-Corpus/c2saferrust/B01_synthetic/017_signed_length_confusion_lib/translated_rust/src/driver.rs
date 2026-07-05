use std::ffi::CString;

use std::ffi::CStr;

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &CStr) {
    println!("{}", line.to_string_lossy());
}

#[no_mangle]
pub fn driver(data: i32) {
    let source = "A".repeat(99);
    let mut dest = String::new();

    if (0..100).contains(&data) {
        let count = data as usize;
        dest.push_str(&source[..count]);
    }

    let dest_c = CString::new(dest).unwrap();
    printLine(dest_c.as_c_str());
}

