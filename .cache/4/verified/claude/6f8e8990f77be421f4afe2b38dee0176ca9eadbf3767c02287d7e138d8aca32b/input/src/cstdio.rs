//! Minimal, dependency-free bindings to the pieces of `<stdio.h>` that the
//! translated C sources use.

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...);`
    pub(crate) unsafe fn printf(format: *const c_char, ...) -> c_int;
}
