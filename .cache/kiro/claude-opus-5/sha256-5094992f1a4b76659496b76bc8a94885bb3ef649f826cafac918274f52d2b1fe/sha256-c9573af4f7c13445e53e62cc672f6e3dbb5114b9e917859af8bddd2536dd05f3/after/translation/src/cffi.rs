//! Declarations of the C library functions that the translation relies on.
//!
//! Formatting, string->number conversion and I/O are delegated to libc so that
//! the produced bytes are identical to the original C build.

use core::ffi::{c_char, c_int, c_void};

pub type FILE = c_void;

pub const EOF: c_int = -1;
pub const STDIN_FILENO: c_int = 0;
pub const O_RDONLY: c_int = 0;
pub const ERANGE: c_int = 34;

#[repr(C)]
pub struct timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn strerror(errnum: c_int) -> *mut c_char;

    pub fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
    pub fn vsnprintf(
        s: *mut c_char,
        n: usize,
        format: *const c_char,
        ap: *mut crate::valist::VaListTag,
    ) -> c_int;

    pub fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
    pub fn strtoll(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;

    pub fn __errno_location() -> *mut c_int;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, stream: *mut FILE) -> usize;
    pub fn fgetc(stream: *mut FILE) -> c_int;

    pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;

    pub fn getpid() -> c_int;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;
    pub fn sched_yield() -> c_int;

    pub static stdin: *mut FILE;
}

#[inline]
pub unsafe fn errno() -> c_int {
    unsafe { *__errno_location() }
}

#[inline]
pub unsafe fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v };
}

/// `strlen()`
#[inline]
pub unsafe fn c_strlen(s: *const c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

/// `memcmp()` restricted to the sign of the result, as used by the C code.
#[inline]
pub unsafe fn c_memcmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    unsafe {
        let mut i = 0usize;
        while i < n {
            let x = *(a.add(i) as *const u8);
            let y = *(b.add(i) as *const u8);
            if x != y {
                return x as c_int - y as c_int;
            }
            i += 1;
        }
        0
    }
}

/// `memchr()`
#[inline]
pub unsafe fn c_memchr(s: *const c_char, c: u8, n: usize) -> *const c_char {
    unsafe {
        let mut i = 0usize;
        while i < n {
            if *(s.add(i) as *const u8) == c {
                return s.add(i);
            }
            i += 1;
        }
        core::ptr::null()
    }
}

/// `strcmp()` against a byte-string literal (including the NUL).
#[inline]
pub unsafe fn c_streq(s: *const c_char, lit: &[u8]) -> bool {
    unsafe {
        let mut i = 0usize;
        loop {
            let a = *(s.add(i) as *const u8);
            let b = lit[i];
            if a != b {
                return false;
            }
            if b == 0 {
                return true;
            }
            i += 1;
        }
    }
}

/// `strchr()`
#[inline]
pub unsafe fn c_strchr(s: *const c_char, c: u8) -> *const c_char {
    unsafe {
        let mut p = s;
        loop {
            let v = *(p as *const u8);
            if v == c {
                return p;
            }
            if v == 0 {
                return core::ptr::null();
            }
            p = p.add(1);
        }
    }
}
