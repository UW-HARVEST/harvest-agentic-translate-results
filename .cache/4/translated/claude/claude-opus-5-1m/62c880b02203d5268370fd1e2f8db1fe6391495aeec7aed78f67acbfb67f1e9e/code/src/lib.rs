//! Rust translation of the C library in `c_src/` (an amalgamation of `stb_ds.h`
//! plus a small test driver).
//!
//! The translation is deliberately literal: every quirk of the original C code
//! (including integer-promotion driven sign extension in the SipHash inner
//! loop, uninitialised fields, and the `printf` of a whole struct in
//! `sh_geti`) is reproduced bit for bit so that the resulting `cdylib` is a
//! drop-in replacement for the C shared object.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod harness;
pub mod stb_ds;

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings
//
// `STBDS_REALLOC(c,p,s)` is `realloc(p,s)` and `STBDS_FREE(c,p)` is `free(p)`
// in the C source, so the real libc allocator must be used (blocks may be
// handed back and forth across the FFI boundary).
// ---------------------------------------------------------------------------
extern "C" {
    pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn abort() -> !;
    pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
}

/// `STBDS_ASSERT` == `assert` in the C source (the CMake build does not define
/// `NDEBUG`, so the assertions are live).
#[inline]
pub(crate) fn stbds_assert(cond: bool, msg: &str) {
    if !cond {
        unsafe {
            write(2, msg.as_ptr() as *const c_void, msg.len());
            abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Small C runtime helpers, re-implemented so the crate has no non-libc deps.
// ---------------------------------------------------------------------------

/// `strlen`
#[inline]
pub(crate) unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// `0 == strcmp(a, b)`
#[inline]
pub(crate) unsafe fn c_str_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut i: usize = 0;
    loop {
        let ca = *(a.add(i)) as u8;
        let cb = *(b.add(i)) as u8;
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        i += 1;
    }
}

/// `0 == memcmp(a, b, n)`
#[inline]
pub(crate) unsafe fn c_mem_eq(a: *const c_void, b: *const c_void, n: usize) -> bool {
    let pa = a as *const u8;
    let pb = b as *const u8;
    let mut i: usize = 0;
    while i < n {
        if *pa.add(i) != *pb.add(i) {
            return false;
        }
        i += 1;
    }
    true
}

/// `memmove`
#[inline]
pub(crate) unsafe fn c_memmove(dst: *mut c_void, src: *const c_void, n: usize) {
    core::ptr::copy(src as *const u8, dst as *mut u8, n);
}

/// `memcpy`
#[inline]
pub(crate) unsafe fn c_memcpy(dst: *mut c_void, src: *const c_void, n: usize) {
    core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, n);
}

/// `memset(dst, 0, n)`
#[inline]
pub(crate) unsafe fn c_memzero(dst: *mut c_void, n: usize) {
    core::ptr::write_bytes(dst as *mut u8, 0, n);
}
