//! Minimal declarations of the C standard library entry points used by the
//! original C sources.
//!
//! The C library relies on libc's stdio for all of its output. In order to
//! produce byte-identical output (including identical stream buffering
//! semantics, `%s`/`%d` formatting and glibc's `(null)` rendering for null
//! `%s` arguments) the translation calls straight into the very same libc
//! routines instead of re-implementing them on top of Rust's `std::io`.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};

/// Opaque stand-in for C's `FILE`.
pub type FILE = c_void;

unsafe extern "C" {
    /// libc's `stderr` stream (a data symbol in glibc).
    pub static mut stderr: *mut FILE;

    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn getenv(name: *const c_char) -> *mut c_char;
    pub fn atoi(nptr: *const c_char) -> c_int;

    pub fn fopen(pathname: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;

    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;

    #[link_name = "printf"]
    pub fn c_printf(format: *const c_char, ...) -> c_int;

    #[link_name = "fprintf"]
    pub fn c_fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
}
