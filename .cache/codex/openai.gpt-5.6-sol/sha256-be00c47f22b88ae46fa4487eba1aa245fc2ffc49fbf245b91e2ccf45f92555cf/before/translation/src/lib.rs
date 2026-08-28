use std::ffi::{c_char, c_void};
use std::ptr;

unsafe extern "C" {
    fn strlen(value: *const c_char) -> usize;
    fn malloc(size: usize) -> *mut c_void;
    fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
}

/// Duplicates a C string using the platform C allocator.
///
/// # Safety
///
/// `value` must be null or point to a readable, NUL-terminated byte sequence.
/// The caller owns a non-null result and must release it with C `free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(value: *const c_char) -> *mut c_char {
    if value.is_null() {
        return ptr::null_mut();
    }

    let length = unsafe { strlen(value) } + 1;
    let duplicate = unsafe { malloc(length) }.cast::<c_char>();
    if duplicate.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        memcpy(duplicate.cast(), value.cast(), length);
    }
    duplicate
}
