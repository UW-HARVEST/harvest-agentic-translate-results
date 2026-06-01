use std::ffi::c_char;
use std::ptr;

extern "C" {
    fn malloc(size: libc::size_t) -> *mut libc::c_void;
    fn strlen(s: *const c_char) -> libc::size_t;
    fn memcpy(
        dest: *mut libc::c_void,
        src: *const libc::c_void,
        n: libc::size_t,
    ) -> *mut libc::c_void;
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
        newstr as *mut libc::c_void,
        str as *const libc::c_void,
        len,
    );
    newstr
}
