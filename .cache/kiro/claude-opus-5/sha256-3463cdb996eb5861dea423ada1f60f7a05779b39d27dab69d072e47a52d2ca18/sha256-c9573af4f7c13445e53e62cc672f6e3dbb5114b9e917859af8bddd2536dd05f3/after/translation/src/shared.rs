//! Translation of `c_src/include/shared.h`.
//!
//! `shared.h` defines these three helpers with external linkage directly in
//! the header, so the compiled shared object exports them.

use core::ffi::{c_char, c_void};

use crate::cbind::*;

/// `void *os_calloc(size_t num, size_t size)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = unsafe { calloc(num, size) };
    if out.is_null() {
        fputs_stderr(b"Memory allocation failed in os_calloc\0");
        unsafe { exit(EXIT_FAILURE) };
    }
    out
}

/// `void *os_realloc(void *ptr, size_t new_size)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    let out = unsafe { realloc(ptr, new_size) };
    if out.is_null() {
        fputs_stderr(b"Memory allocation failed in os_realloc\0");
        unsafe { exit(EXIT_FAILURE) };
    }
    out
}

/// `char *os_strdup(const char *str)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        fputs_stderr(b"NULL string passed to os_strdup\0");
        unsafe { exit(EXIT_FAILURE) };
    }
    let dup = unsafe { strdup(str_) };
    if dup.is_null() {
        fputs_stderr(b"Memory allocation failed in os_strdup\0");
        unsafe { exit(EXIT_FAILURE) };
    }
    dup
}
