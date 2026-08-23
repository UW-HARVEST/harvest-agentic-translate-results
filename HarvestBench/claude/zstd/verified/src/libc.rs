//! Minimal libc bindings used by the translated zstd code.
//! Using the platform allocator (rather than Rust's) keeps allocation
//! behaviour identical to the C library, and using libc's `qsort` keeps
//! sort tie-breaking byte-identical.
#![allow(non_camel_case_types)]

use core::ffi::c_void;

pub type size_t = usize;
pub type c_int = i32;

extern "C" {
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn calloc(nmemb: size_t, size: size_t) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: size_t) -> c_int;
    pub fn qsort(
        base: *mut c_void,
        nmemb: size_t,
        size: size_t,
        compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
    );
    pub fn qsort_r(
        base: *mut c_void,
        nmemb: size_t,
        size: size_t,
        compar: Option<unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int>,
        arg: *mut c_void,
    );
    pub fn clock() -> i64;
}

/// `ZSTD_memcpy`
#[inline(always)]
pub unsafe fn ZSTD_memcpy(d: *mut c_void, s: *const c_void, l: size_t) {
    if l != 0 {
        core::ptr::copy_nonoverlapping(s as *const u8, d as *mut u8, l);
    }
}

/// `ZSTD_memmove`
#[inline(always)]
pub unsafe fn ZSTD_memmove(d: *mut c_void, s: *const c_void, l: size_t) {
    if l != 0 {
        core::ptr::copy(s as *const u8, d as *mut u8, l);
    }
}

/// `ZSTD_memset`
#[inline(always)]
pub unsafe fn ZSTD_memset(p: *mut c_void, v: c_int, l: size_t) {
    if l != 0 {
        core::ptr::write_bytes(p as *mut u8, v as u8, l);
    }
}
