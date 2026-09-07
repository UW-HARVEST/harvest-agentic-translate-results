//! Minimal bindings to the host C standard I/O library.
//!
//! The translated library must produce *byte-identical* output to the original
//! C code. Output ordering between `stdout` and `stderr` depends on the exact
//! buffering behaviour of the C runtime (`stdout` is line-buffered on a TTY but
//! fully buffered when redirected to a file or pipe, `stderr` is unbuffered).
//! Re-implementing this on top of Rust's own `std::io` buffers would interleave
//! differently, so we call straight through to the platform's `stdio` instead.

use std::ffi::{c_char, c_int};

/// Opaque stand-in for C's `FILE`.
///
/// Declared as a zero-sized `repr(C)` struct so that `*mut FILE` is a plain
/// thin pointer that is ABI-identical to the C type, while remaining
/// impossible to dereference or construct by accident from Rust.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    /// `extern FILE *stderr;` from `<stdio.h>`.
    pub static mut stderr: *mut FILE;

    pub fn printf(format: *const c_char, ...) -> c_int;
    pub fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;

    pub fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fgets(buf: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    pub fn ferror(stream: *mut FILE) -> c_int;
}
