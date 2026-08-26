use std::ffi::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let len = libc::strlen(str) + 1;
        let newstr = libc::malloc(len) as *mut c_char;
        if newstr.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(str, newstr, len);
        newstr
    }
}
