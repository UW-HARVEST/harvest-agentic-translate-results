//! Small shims over the C standard library facilities that libpng uses
//! directly (stdio, malloc, gmtime, ...).
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_long, c_void};

#[repr(C)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

pub type time_t = i64;

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    pub fn fclose(f: *mut c_void) -> c_int;
    pub fn fread(buf: *mut c_void, size: usize, n: usize, f: *mut c_void) -> usize;
    pub fn fwrite(buf: *const c_void, size: usize, n: usize, f: *mut c_void) -> usize;
    pub fn fflush(f: *mut c_void) -> c_int;
    pub fn ferror(f: *mut c_void) -> c_int;
    pub fn fprintf(f: *mut c_void, fmt: *const c_char, ...) -> c_int;

    pub fn gmtime(t: *const time_t) -> *mut tm;

    #[link_name = "stderr"]
    pub static mut stderr_ptr: *mut c_void;
}

/// `memcpy`
#[inline]
pub unsafe fn memcpy(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        core::ptr::copy_nonoverlapping(src, dst, n);
    }
}

/// `memmove`
#[inline]
pub unsafe fn memmove(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        core::ptr::copy(src, dst, n);
    }
}

/// `memset`
#[inline]
pub unsafe fn memset(dst: *mut u8, v: u8, n: usize) {
    if n != 0 {
        core::ptr::write_bytes(dst, v, n);
    }
}

/// `memcmp`
#[inline]
pub unsafe fn memcmp(a: *const u8, b: *const u8, n: usize) -> c_int {
    for i in 0..n {
        let x = *a.add(i);
        let y = *b.add(i);
        if x != y {
            return x as c_int - y as c_int;
        }
    }
    0
}

/// `strlen`
#[inline]
pub unsafe fn strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// `strcmp`
#[inline]
pub unsafe fn strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0usize;
    loop {
        let x = *a.add(i) as u8;
        let y = *b.add(i) as u8;
        if x != y {
            return x as c_int - y as c_int;
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
}
