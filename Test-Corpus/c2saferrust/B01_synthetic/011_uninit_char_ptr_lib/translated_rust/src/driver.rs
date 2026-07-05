


use std::ffi::CStr;

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn bad() {
    let data = "";
    printLine(data);
}

#[no_mangle]
pub fn good() {
    let data = "string";
    printLine(data);
}

#[no_mangle]
pub fn driver(use_good: bool) {
    if use_good {
        good();
    } else {
        bad();
    }
}

