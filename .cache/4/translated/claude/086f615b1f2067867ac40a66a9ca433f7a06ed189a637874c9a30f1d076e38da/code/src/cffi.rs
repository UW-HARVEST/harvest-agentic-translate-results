//! Raw declarations for the subset of the C standard library used by the
//! original C sources.
//!
//! The translation deliberately calls straight through to the platform libc
//! (rather than using Rust's `std::io` / `std::alloc`) so that:
//!   * buffering behaviour of `stdout`/`stderr` is bit-for-bit the same as the
//!     C library's,
//!   * `printf`/`fprintf` conversion semantics (including things like `%s`
//!     with a NULL pointer) match exactly,
//!   * memory returned from `create_task_manager` is `malloc`-owned, exactly
//!     as in C, so callers may `free()` it and `destroy_task_manager` behaves
//!     identically.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

/// Opaque stdio `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

unsafe extern "C" {
    /// glibc/musl export `stderr` as a data symbol of type `FILE *`.
    pub static mut stderr: *mut FILE;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn printf(fmt: *const c_char, ...) -> c_int;

    pub fn getenv(name: *const c_char) -> *mut c_char;
    pub fn atoi(s: *const c_char) -> c_int;

    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

/// `EXIT_FAILURE` from `<stdlib.h>`.
pub const EXIT_FAILURE: c_int = 1;
