//! Raw bindings to the libc entities that the original C library uses.
//!
//! The C sources call directly into glibc for every string / stdio / memory
//! operation.  To guarantee byte-for-byte identical behaviour (including
//! `errno` evolution, `printf` formatting and `strtol`/`atoi` corner cases) the
//! translation calls the very same functions instead of re-implementing them.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

/// Opaque `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

/// `struct timespec`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timespec {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

/// `struct timeval`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timeval {
    pub tv_sec: c_long,
    pub tv_usec: c_long,
}

/// glibc x86-64 `struct stat` (144 bytes, `st_mtim` at offset 88).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: c_ulong,
    pub st_ino: c_ulong,
    pub st_nlink: c_ulong,
    pub st_mode: c_uint,
    pub st_uid: c_uint,
    pub st_gid: c_uint,
    pub __pad0: c_int,
    pub st_rdev: c_ulong,
    pub st_size: c_long,
    pub st_blksize: c_long,
    pub st_blocks: c_long,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [c_long; 3],
}

/// glibc `struct tm` (56 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
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

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

pub const EXIT_FAILURE: c_int = 1;

extern "C" {
    pub static mut stderr: *mut FILE;

    // <stdlib.h>
    pub fn calloc(num: usize, size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn exit(status: c_int) -> !;
    pub fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;

    // <stdio.h>
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fseek(stream: *mut FILE, offset: c_long, whence: c_int) -> c_int;
    pub fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    pub fn feof(stream: *mut FILE) -> c_int;
    pub fn clearerr(stream: *mut FILE);
    pub fn fileno(stream: *mut FILE) -> c_int;
    pub fn perror(s: *const c_char);
    pub fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;

    // <string.h>
    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    pub fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;

    // <errno.h>
    pub fn __errno_location() -> *mut c_int;

    // <sys/stat.h>
    pub fn fstat(fd: c_int, buf: *mut stat) -> c_int;

    // <sys/select.h>
    pub fn select(
        nfds: c_int,
        readfds: *mut c_void,
        writefds: *mut c_void,
        exceptfds: *mut c_void,
        timeout: *mut timeval,
    ) -> c_int;
}

/// `errno`
#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

/// glibc's `atoi` is an inline wrapper around `strtol` with the result
/// truncated to `int` -- exactly what the compiled C does.
#[inline]
pub unsafe fn atoi(nptr: *const c_char) -> c_int {
    strtol(nptr, core::ptr::null_mut(), 10) as c_int
}

/// Helper: pointer to the NUL-terminated bytes of a byte-string literal.
#[inline]
pub const fn cs(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
