//! Minimal declarations of the C standard library entry points used by the
//! original C sources. The translation calls straight through to libc so that
//! allocation, tokenisation, formatting and stdio behaviour (including
//! diagnostics written to `stderr` and `errno` values) are byte-for-byte
//! identical to the C library.

use core::ffi::{c_char, c_int, c_long, c_void};

/// Opaque stand-in for C's `FILE`.
#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub static mut stderr: *mut FILE;

    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strtok_r(
        s: *mut c_char,
        delim: *const c_char,
        saveptr: *mut *mut c_char,
    ) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn atoi(s: *const c_char) -> c_int;

    pub fn perror(s: *const c_char);
    pub fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    pub fn snprintf(str: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;

    fn __errno_location() -> *mut c_int;
}

/// `errno` as an rvalue.
#[inline]
pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}

/// `EINVAL` on Linux.
pub const EINVAL: c_int = 22;

/// `EXIT_SUCCESS` from `<stdlib.h>`.
pub const EXIT_SUCCESS: c_int = 0;
/// `EXIT_FAILURE` from `<stdlib.h>`.
pub const EXIT_FAILURE: c_int = 1;

/// `sizeof(int)` as C computes it in `malloc(width * sizeof(int))`.
pub const SIZEOF_INT: usize = core::mem::size_of::<c_int>();
/// `sizeof(int*)` as C computes it in `malloc(height * sizeof(int*))`.
pub const SIZEOF_INT_PTR: usize = core::mem::size_of::<*mut c_int>();

/// Reproduces C's `n * sizeof(T)`: the (possibly negative) `int` is converted
/// to `size_t` first, then multiplied with wrapping semantics.
#[inline]
pub fn c_size_mul(n: c_int, elem: usize) -> usize {
    (n as c_long as usize).wrapping_mul(elem)
}

/// `strcat(dst, src)`: append `src` (NUL terminated) at the first NUL of `dst`.
///
/// # Safety
/// Same contract as C's `strcat`.
pub unsafe fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    unsafe {
        let mut end = dst;
        while *end != 0 {
            end = end.add(1);
        }
        let mut s = src;
        loop {
            let b = *s;
            *end = b;
            if b == 0 {
                break;
            }
            end = end.add(1);
            s = s.add(1);
        }
        dst
    }
}
