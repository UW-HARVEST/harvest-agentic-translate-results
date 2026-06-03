use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return std::ptr::null_mut();
    }

    let len = libc::strlen(str) + 1;

    let newstr = libc::malloc(len) as *mut c_char;
    if newstr.is_null() {
        return std::ptr::null_mut();
    }

    libc::memcpy(
        newstr as *mut libc::c_void,
        str as *const libc::c_void,
        len,
    );
    newstr
}
