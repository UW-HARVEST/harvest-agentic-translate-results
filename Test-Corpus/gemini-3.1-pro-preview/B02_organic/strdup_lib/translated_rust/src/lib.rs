use std::ffi::{CStr, c_char};
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return ptr::null_mut();
    }

    let bytes = unsafe { CStr::from_ptr(str) }.to_bytes_with_nul();
    let len = bytes.len();

    let newstr = unsafe { libc::malloc(len) } as *mut c_char;
    if newstr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        ptr::copy_nonoverlapping(str, newstr, len);
    }

    newstr
}
