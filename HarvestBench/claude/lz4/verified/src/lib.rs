// LZ4 v1.10.0 + xxHash 0.6.5 (namespaced LZ4_XXH*) — Rust cdylib transliteration.
// Target: x86_64 little-endian. Produces byte-identical output to the C library.
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// ALLOC(s) => malloc(s)
#[inline]
pub(crate) unsafe fn c_malloc(size: usize) -> *mut u8 {
    malloc(size) as *mut u8
}
// ALLOC_AND_ZERO(s) => calloc(1, s)
#[inline]
pub(crate) unsafe fn c_calloc(size: usize) -> *mut u8 {
    calloc(1, size) as *mut u8
}
// FREEMEM(p) => free(p)
#[inline]
pub(crate) unsafe fn c_free(p: *mut u8) {
    free(p as *mut c_void)
}

pub mod xxhash;
pub mod lz4;
pub mod lz4hc;
pub mod lz4frame;
pub mod lz4file;
