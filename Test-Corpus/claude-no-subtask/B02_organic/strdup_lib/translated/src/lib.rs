use std::ffi::c_char;
use std::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut core::ffi::c_void;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(
        dest: *mut core::ffi::c_void,
        src: *const core::ffi::c_void,
        n: usize,
    ) -> *mut core::ffi::c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return ptr::null_mut();
    }

    let len = strlen(str) + 1;

    let newstr = malloc(len) as *mut c_char;
    if newstr.is_null() {
        return ptr::null_mut();
    }

    memcpy(
        newstr as *mut core::ffi::c_void,
        str as *const core::ffi::c_void,
        len,
    );
    newstr
}
