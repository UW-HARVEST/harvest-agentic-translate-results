//! Declarations for the C runtime routines used by the original library.
//!
//! The translation deliberately calls straight through to libc rather than
//! reimplementing formatting or allocation in Rust. `charinbuf` writes to the
//! process' C `stdout`, and buffers returned by `create_buffer` are expected to
//! be releasable with `free()`, so both must be the genuine libc versions in
//! order for the observable behaviour to stay byte-identical.

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...)`
    pub fn printf(format: *const c_char, ...) -> c_int;

    /// `void *malloc(size_t size)`
    pub fn malloc(size: usize) -> *mut c_void;

    /// `void free(void *ptr)`
    pub fn free(ptr: *mut c_void);

    /// `size_t strlen(const char *s)`
    pub fn strlen(s: *const c_char) -> usize;

    /// `char *strcpy(char *dest, const char *src)`
    pub fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;

    /// `void *memchr(const void *s, int c, size_t n)`
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}
