//! Translation of `c_src/include/shared.h`.
//!
//! `shared.h` defines three *non-static* functions in the header itself, so the
//! translation unit that includes it (`read-alert.c`) emits them as global
//! symbols in the shared object.  They are part of the public ABI.

use core::ffi::{c_char, c_void};

use crate::cbits::*;

/// `#define OS_MAXSTR 1024`
pub const OS_MAXSTR: usize = 1024;

/// ```c
/// void *os_calloc(size_t num, size_t size) {
///     void *out = calloc(num, size);
///     if (!out) {
///         fprintf(stderr, "Memory allocation failed in os_calloc");
///         exit(EXIT_FAILURE);
///     }
///     return out;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = calloc(num, size);
    if out.is_null() {
        fprintf(
            stderr,
            c"Memory allocation failed in os_calloc".as_ptr(),
        );
        exit(EXIT_FAILURE);
    }
    out
}

/// ```c
/// void *os_realloc(void *ptr, size_t new_size) {
///     void *out = realloc(ptr, new_size);
///     if (!out) {
///         fprintf(stderr, "Memory allocation failed in os_realloc");
///         exit(EXIT_FAILURE);
///     }
///     return out;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    let out = realloc(ptr, new_size);
    if out.is_null() {
        fprintf(
            stderr,
            c"Memory allocation failed in os_realloc".as_ptr(),
        );
        exit(EXIT_FAILURE);
    }
    out
}

/// ```c
/// char *os_strdup(const char *str) {
///     if (!str) {
///         fprintf(stderr, "NULL string passed to os_strdup");
///         exit(EXIT_FAILURE);
///     }
///     char *dup = strdup(str);
///     if (!dup) {
///         fprintf(stderr, "Memory allocation failed in os_strdup");
///         exit(EXIT_FAILURE);
///     }
///     return dup;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        fprintf(stderr, c"NULL string passed to os_strdup".as_ptr());
        exit(EXIT_FAILURE);
    }
    let dup = strdup(str);
    if dup.is_null() {
        fprintf(
            stderr,
            c"Memory allocation failed in os_strdup".as_ptr(),
        );
        exit(EXIT_FAILURE);
    }
    dup
}
