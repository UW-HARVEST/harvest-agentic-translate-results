use std::ffi::{c_char, c_void};
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return ptr::null_mut();
    }

    let len = unsafe { strlen(str) }.wrapping_add(1);
    let newstr = unsafe { malloc(len) }.cast::<c_char>();
    if newstr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        ptr::copy_nonoverlapping(str.cast::<u8>(), newstr.cast::<u8>(), len);
    }
    newstr
}
