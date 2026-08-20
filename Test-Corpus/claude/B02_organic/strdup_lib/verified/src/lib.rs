//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (must match the C shared library exactly):
//!   * `char *custom_strdup(const char *str);`
//!
//! The returned buffer is allocated with the C allocator (`malloc`), exactly as
//! in the C original, so that callers may release it with `free()`.

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
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    // if(!str) return NULL;
    if str.is_null() {
        return std::ptr::null_mut();
    }

    // len = strlen(str) + 1;
    // NOTE: reproduced verbatim, including the C original's unchecked
    // `+ 1` overflow behaviour on pathological inputs.
    let len: usize = unsafe { strlen(str) }.wrapping_add(1);

    // newstr = malloc(len);
    let newstr = unsafe { malloc(len) } as *mut c_char;
    if newstr.is_null() {
        return std::ptr::null_mut();
    }

    // memcpy(newstr, str, len);
    unsafe {
        memcpy(newstr as *mut c_void, str as *const c_void, len);
    }

    newstr
}
