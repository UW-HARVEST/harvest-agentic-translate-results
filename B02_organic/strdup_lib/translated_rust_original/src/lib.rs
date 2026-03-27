use std::ffi::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return ptr::null_mut();
    }

    let len = unsafe { libc::strlen(str) } + 1;

    let newstr = unsafe { libc::malloc(len) } as *mut c_char;
    if newstr.is_null() {
        return ptr::null_mut();
    }

    unsafe { ptr::copy_nonoverlapping(str, newstr, len) };
    newstr
}
