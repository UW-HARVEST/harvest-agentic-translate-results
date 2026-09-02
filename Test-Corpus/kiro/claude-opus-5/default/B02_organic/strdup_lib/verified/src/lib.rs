//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (matches `nm -D` on the C `libdriver.so`):
//!   * `custom_strdup`
//!
//! The header `include/lib.h` declares no namespace-renaming macros, so the
//! source-level names are also the final linker symbol names.

use std::ffi::{c_char, c_void};

// The C implementation hands back a buffer obtained from `malloc`, which makes
// the allocator part of the observable ABI: callers are expected to release the
// result with `free`. Rust's own allocator is not guaranteed to be compatible
// with `free`, so the platform allocator is used directly instead. These are
// declared locally to keep the crate dependency-free.
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
///
/// The order of the checks (null input first, then allocation failure) is
/// preserved exactly, as is the `strlen(str) + 1` length computation including
/// its wrapping behaviour on overflow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return std::ptr::null_mut();
    }

    // `strlen(str) + 1`: matches the C exactly, wrapping like `size_t` would.
    let len: usize = unsafe { strlen(str) }.wrapping_add(1);

    let newstr = unsafe { malloc(len) };
    if newstr.is_null() {
        return std::ptr::null_mut();
    }

    unsafe { memcpy(newstr, str as *const c_void, len) };
    newstr as *mut c_char
}
