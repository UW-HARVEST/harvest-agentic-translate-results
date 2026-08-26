//! Minimal declarations of the C library functions used by the translation.
//! Using the platform C library guarantees byte-identical formatting and
//! numeric conversion behaviour.
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_long, c_void};

#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

/* x86-64 SysV va_list */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VaListTag {
    pub gp_offset: u32,
    pub fp_offset: u32,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}

pub type va_list = *mut VaListTag;

#[inline]
pub unsafe fn va_arg_gp<T: Copy>(ap: va_list) -> T {
    debug_assert!(std::mem::size_of::<T>() <= 8);
    let p: *mut u8;
    if (*ap).gp_offset <= 48 - 8 {
        p = ((*ap).reg_save_area as *mut u8).add((*ap).gp_offset as usize);
        (*ap).gp_offset += 8;
    } else {
        p = (*ap).overflow_arg_area as *mut u8;
        (*ap).overflow_arg_area = p.add(8) as *mut c_void;
    }
    std::ptr::read_unaligned(p as *const T)
}

#[inline]
pub unsafe fn va_arg_fp(ap: va_list) -> f64 {
    let p: *mut u8;
    if (*ap).fp_offset <= 176 - 16 {
        p = ((*ap).reg_save_area as *mut u8).add((*ap).fp_offset as usize);
        (*ap).fp_offset += 16;
    } else {
        p = (*ap).overflow_arg_area as *mut u8;
        (*ap).overflow_arg_area = p.add(8) as *mut c_void;
    }
    std::ptr::read_unaligned(p as *const f64)
}

/* A copy of a va_list value (C's va_copy on x86-64 copies the struct). */
#[inline]
pub unsafe fn va_copy(ap: va_list, dst: *mut VaListTag) -> va_list {
    *dst = *ap;
    dst
}

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;
    pub fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64;

    pub fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn vsnprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ap: va_list) -> c_int;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
    pub fn fgetc(f: *mut FILE) -> c_int;

    pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;

    pub fn qsort(
        base: *mut c_void,
        nmemb: usize,
        size: usize,
        compar: unsafe extern "C" fn(*const c_void, *const c_void) -> c_int,
    );

    pub fn getpid() -> c_int;
    pub fn sched_yield() -> c_int;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;

    pub fn __errno_location() -> *mut c_int;

    /// glibc's assertion-failure hook, so live `assert()`s in the C sources
    /// abort with the same message here.
    pub fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: u32,
        function: *const c_char,
    ) -> !;

    pub static stdin: *mut FILE;
}

#[repr(C)]
pub struct timeval {
    pub tv_sec: c_long,
    pub tv_usec: c_long,
}

pub const EOF: c_int = -1;
pub const ERANGE: c_int = 34;
pub const O_RDONLY: c_int = 0;
pub const STDIN_FILENO: c_int = 0;
pub const HUGE_VAL: f64 = f64::INFINITY;

#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

#[inline]
pub unsafe fn set_errno(v: c_int) {
    *__errno_location() = v;
}
