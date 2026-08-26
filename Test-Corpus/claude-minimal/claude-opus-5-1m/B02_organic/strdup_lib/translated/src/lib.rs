use libc::{c_char, malloc, memcpy, size_t, strlen};
use std::ptr;

/// Duplicates a null-terminated C string by allocating a new buffer with `malloc`
/// and copying the contents (including the trailing null byte).
///
/// # Safety
///
/// `str` must either be null or point to a valid null-terminated C string.
/// The returned pointer (if non-null) is allocated with `malloc` and must be
/// freed with `free`.
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return ptr::null_mut();
    }

    let len: size_t = strlen(str) + 1;

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
