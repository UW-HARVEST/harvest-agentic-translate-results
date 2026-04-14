use std::ffi::{c_char, c_void, CStr};
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return ptr::null_mut();
    }

    let bytes = unsafe { CStr::from_ptr(str) }.to_bytes_with_nul();
    let len = bytes.len();

    let newstr = unsafe { malloc(len) } as *mut c_char;
    if newstr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        memcpy(newstr as *mut c_void, str as *const c_void, len);
    }

    newstr
}
