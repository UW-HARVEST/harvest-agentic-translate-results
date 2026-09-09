//! Minimal libc / system bindings (the crate has no external dependencies).
#![allow(dead_code, non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_uint, c_void};

pub type size_t = usize;
pub type ssize_t = isize;
pub type off_t = i64;
pub type mode_t = u32;

extern "C" {
    pub fn malloc(n: size_t) -> *mut c_void;
    pub fn calloc(n: size_t, s: size_t) -> *mut c_void;
    pub fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;
    pub fn exit(code: c_int) -> !;

    pub fn memcpy(d: *mut c_void, s: *const c_void, n: size_t) -> *mut c_void;
    pub fn memmove(d: *mut c_void, s: *const c_void, n: size_t) -> *mut c_void;
    pub fn memset(d: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: size_t) -> c_int;
    pub fn strlen(s: *const c_char) -> size_t;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: size_t) -> c_int;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;

    pub fn sysconf(name: c_int) -> c_long;
    pub fn posix_memalign(memptr: *mut *mut c_void, align: size_t, size: size_t) -> c_int;
    pub fn mmap(
        addr: *mut c_void,
        len: size_t,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: off_t,
    ) -> *mut c_void;
    pub fn munmap(addr: *mut c_void, len: size_t) -> c_int;
    pub fn mprotect(addr: *mut c_void, len: size_t, prot: c_int) -> c_int;
    pub fn mlock(addr: *const c_void, len: size_t) -> c_int;
    pub fn munlock(addr: *const c_void, len: size_t) -> c_int;
    pub fn madvise(addr: *mut c_void, len: size_t, advice: c_int) -> c_int;

    pub fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    pub fn close(fd: c_int) -> c_int;
    pub fn poll(fds: *mut c_void, nfds: c_uint, timeout: c_int) -> c_int;
    pub fn getpid() -> i32;
    pub fn usleep(usec: c_uint) -> c_int;
    pub fn nanosleep(req: *const c_void, rem: *mut c_void) -> c_int;
    pub fn __errno_location() -> *mut c_int;
    pub fn syscall(num: c_long, ...) -> c_long;
    pub fn explicit_bzero(s: *mut c_void, n: size_t);
    pub fn getentropy(buf: *mut c_void, len: size_t) -> c_int;
}

#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

#[inline]
pub unsafe fn set_errno(v: c_int) {
    *__errno_location() = v;
}

pub const EINTR: c_int = 4;
pub const EIO: c_int = 5;
pub const ENOSYS: c_int = 38;
pub const EPERM: c_int = 1;
pub const ENOMEM: c_int = 12;
pub const EINVAL: c_int = 22;
pub const ERANGE: c_int = 34;
pub const EAGAIN: c_int = 11;
pub const ENFILE: c_int = 23;
pub const EMFILE: c_int = 24;
pub const EEXIST: c_int = 17;

pub const O_RDONLY: c_int = 0;
pub const O_CLOEXEC: c_int = 0o2000000;

pub const PROT_NONE: c_int = 0;
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;
pub const MAP_PRIVATE: c_int = 0x02;
pub const MAP_ANON: c_int = 0x20;
pub const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;

pub const MADV_DONTDUMP: c_int = 16;
pub const MADV_DODUMP: c_int = 17;

pub const _SC_PAGESIZE: c_int = 30;

pub const SYS_getrandom: c_long = 318;
