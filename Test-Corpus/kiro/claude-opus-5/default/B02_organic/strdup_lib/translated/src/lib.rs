//! Rust translation of `c_src/src/lib.c`.
//!
//! The single exported entry point is `custom_strdup`, a re-implementation of
//! POSIX `strdup`. The header (`c_src/include/lib.h`) declares it without any
//! namespacing macro, so the final linker symbol is plainly `custom_strdup`.
//!
//! The buffer is deliberately obtained from the C allocator (`malloc`) rather
//! than Rust's allocator: callers of the original library release the result
//! with `free`, so the allocator pair must be preserved.

use std::ffi::c_char;
use std::ffi::c_void;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

/// Duplicates the NUL-terminated string `str` into a freshly `malloc`ed buffer.
///
/// Returns `NULL` if `str` is `NULL` or if the allocation fails, mirroring the
/// C original exactly (including the order of the two checks).
///
/// # Safety
///
/// `str` must either be `NULL` or point to a valid NUL-terminated C string.
/// The returned pointer, when non-`NULL`, is owned by the caller and must be
/// released with `free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    // if(!str) return NULL;
    if str.is_null() {
        return std::ptr::null_mut();
    }

    // len = strlen(str) + 1;
    let len = unsafe { strlen(str) } + 1;

    // newstr = malloc(len); if(!newstr) return NULL;
    let newstr = unsafe { malloc(len) } as *mut c_char;
    if newstr.is_null() {
        return std::ptr::null_mut();
    }

    // memcpy(newstr, str, len);
    unsafe { std::ptr::copy_nonoverlapping(str, newstr, len) };

    newstr
}

/// Length of a NUL-terminated C string, excluding the terminator.
///
/// # Safety
///
/// `s` must be a non-null pointer to a NUL-terminated C string.
unsafe fn strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while unsafe { *s.add(n) } != 0 {
        n += 1;
    }
    n
}
