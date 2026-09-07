//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C `libdriver.so`):
//!   * `custom_strdup`
//!
//! The C implementation allocates the returned buffer with `malloc`, so callers
//! are expected to release it with `free`. To remain ABI/allocator compatible we
//! call the platform `malloc`/`memcpy`/`strlen` directly rather than using the
//! Rust global allocator.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

/// Translation of:
///
/// ```c
/// char *custom_strdup(const char *str)
/// {
///   size_t len;
///   char *newstr;
///
///   if(!str)
///     return (char *)NULL;
///
///   len = strlen(str) + 1;
///
///   newstr = malloc(len);
///   if(!newstr)
///     return (char *)NULL;
///
///   memcpy(newstr, str, len);
///   return newstr;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        return std::ptr::null_mut();
    }

    // len = strlen(str) + 1 (wrapping, exactly like the C `size_t` arithmetic)
    let len: usize = unsafe { strlen(str_) }.wrapping_add(1);

    let newstr = unsafe { malloc(len) } as *mut c_char;
    if newstr.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        memcpy(newstr as *mut c_void, str_ as *const c_void, len);
    }
    newstr
}
