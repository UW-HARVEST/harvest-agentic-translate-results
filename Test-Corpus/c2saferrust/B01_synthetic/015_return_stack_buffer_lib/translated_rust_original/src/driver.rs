



use std::ffi::CStr;

use std::ffi::CString;

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &CStr) {
    println!("{}", line.to_string_lossy());
}

fn helperBad() -> String {
    "helperBad string".to_string()
}

#[no_mangle]
pub fn bad() {
    let line = helperBad();
    let c_line = CString::new(line).expect("helperBad() returned a string containing an interior NUL byte");
    printLine(c_line.as_c_str());
}

fn helperGood1() -> &'static CStr {
    CStr::from_bytes_with_nul(b"helperGood1 string\0").unwrap()
}

#[no_mangle]
pub fn good() {
    printLine(helperGood1());
}

#[no_mangle]
pub fn driver(use_good: bool) {
    if use_good {
        good();
    } else {
        bad();
    }
}

