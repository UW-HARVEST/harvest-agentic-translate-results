/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Minimal bindings to the platform C library.
//!
//! The original library relies on `stdio` streams, `malloc`/`free` and the
//! `str*` family. Re-using the very same C entry points (instead of Rust
//! equivalents) keeps the observable behaviour - buffering, interleaving with a
//! caller's own stdio writes, allocator identity, `atoi`/`getenv` semantics -
//! bit-for-bit identical to the C implementation.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};

/// Opaque `FILE`.
#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

pub const EXIT_FAILURE: c_int = 1;

extern "C" {
    pub static mut stderr: *mut FILE;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    pub fn printf(format: *const c_char, ...) -> c_int;

    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn getenv(name: *const c_char) -> *mut c_char;
    pub fn atoi(s: *const c_char) -> c_int;

    pub fn strlen(s: *const c_char) -> usize;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}
