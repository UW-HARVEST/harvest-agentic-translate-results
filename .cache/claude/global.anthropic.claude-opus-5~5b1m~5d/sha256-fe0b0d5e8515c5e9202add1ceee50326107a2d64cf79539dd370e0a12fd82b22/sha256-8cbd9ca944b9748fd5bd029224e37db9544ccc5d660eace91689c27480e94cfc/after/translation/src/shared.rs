//! Translation of `c_src/include/shared.h`.
//!
//! The header defines (not just declares) `os_calloc`, `os_realloc` and
//! `os_strdup` with external linkage, so the shared object exports them.

use core::ffi::{c_char, c_void};

use crate::cbind::*;

/// `#define OS_MAXSTR 1024`
pub const OS_MAXSTR: usize = 1024;

/// `#define os_free(x) if(x){free(x);x=NULL;};`
///
/// Takes a mutable place expression holding a `*mut T`.
macro_rules! os_free {
    ($x:expr) => {{
        if !$x.is_null() {
            $crate::cbind::free($x as *mut core::ffi::c_void);
            $x = core::ptr::null_mut();
        }
    }};
}

/// `#define os_clearnl(x,p) if((p = strrchr(x, '\n')))*p = '\0';`
macro_rules! os_clearnl {
    ($x:expr, $p:expr) => {{
        $p = $crate::cbind::strrchr($x, b'\n' as core::ffi::c_int);
        if !$p.is_null() {
            *$p = 0;
        }
    }};
}

pub(crate) use os_clearnl;
pub(crate) use os_free;

/// ```c
/// void *os_calloc(size_t num, size_t size)
/// ```
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = calloc(num, size);
    if out.is_null() {
        fprintf(stderr, cs(b"Memory allocation failed in os_calloc\0"));
        exit(EXIT_FAILURE);
    }
    out
}

/// ```c
/// void *os_realloc(void *ptr, size_t new_size)
/// ```
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    let out = realloc(ptr, new_size);
    if out.is_null() {
        fprintf(stderr, cs(b"Memory allocation failed in os_realloc\0"));
        exit(EXIT_FAILURE);
    }
    out
}

/// ```c
/// char *os_strdup(const char *str)
/// ```
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        fprintf(stderr, cs(b"NULL string passed to os_strdup\0"));
        exit(EXIT_FAILURE);
    }
    let dup = strdup(str_);
    if dup.is_null() {
        fprintf(stderr, cs(b"Memory allocation failed in os_strdup\0"));
        exit(EXIT_FAILURE);
    }
    dup
}
