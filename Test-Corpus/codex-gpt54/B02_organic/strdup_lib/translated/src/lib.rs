use libc::{malloc, strlen};
use std::ffi::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str_: *const c_char) -> *mut c_char {
    let len;
    let newstr;

    if str_.is_null() {
        return ptr::null_mut();
    }

    len = unsafe { strlen(str_) } + 1;

    newstr = unsafe { malloc(len) as *mut c_char };
    if newstr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        ptr::copy_nonoverlapping(str_, newstr, len);
    }
    newstr
}
