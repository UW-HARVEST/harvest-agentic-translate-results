//! Minimal hand-rolled bindings to the pieces of libc that the original C
//! sources use.
//!
//! The C library hands `malloc`-ed pointers out across its public ABI (for
//! example `matrix_to_string` returns a buffer that the caller releases with
//! `free`), so the translation must use the *same* allocator rather than
//! Rust's. Likewise all diagnostics are routed through the C `stderr` stream
//! and `perror`/`strerror` so that the emitted bytes, their ordering, and the
//! stream buffering behaviour are bit-for-bit identical to the C build.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};

/// Opaque stand-in for C's `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    pub static mut stderr: *mut FILE;

    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strcat(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    pub fn strtok_r(
        s: *mut c_char,
        delim: *const c_char,
        saveptr: *mut *mut c_char,
    ) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn atoi(s: *const c_char) -> c_int;

    pub fn perror(s: *const c_char);
    pub fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;

    pub fn __errno_location() -> *mut c_int;
}

/// `errno`, read at the exact point the C code reads it.
#[inline]
pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}

/// The C `stderr` stream.
#[inline]
pub fn stderr_stream() -> *mut FILE {
    unsafe { stderr }
}

/// `EINVAL` from `<errno.h>` on Linux/glibc.
pub const EINVAL: c_int = 22;

/// `EXIT_SUCCESS` from `<stdlib.h>`.
pub const EXIT_SUCCESS: c_int = 0;

/// `EXIT_FAILURE` from `<stdlib.h>`.
pub const EXIT_FAILURE: c_int = 1;
