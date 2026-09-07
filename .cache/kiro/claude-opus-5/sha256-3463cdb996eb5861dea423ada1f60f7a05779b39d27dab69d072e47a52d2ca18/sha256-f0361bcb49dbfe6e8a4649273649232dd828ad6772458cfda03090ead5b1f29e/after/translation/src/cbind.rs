//! Raw declarations of the libc entities used by the original C sources.
//!
//! The C library is linked against glibc and relies on its exact string /
//! stdio / stat semantics.  Rather than re-implementing those (which risks
//! subtle behavioural drift) we call straight through to the very same
//! functions the C object files referenced.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_ulong, c_void};

/// Opaque `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

/// `struct timeval` (x86_64 linux-gnu).
#[repr(C)]
pub struct timeval {
    pub tv_sec: c_long,
    pub tv_usec: c_long,
}

/// `struct timespec` (x86_64 linux-gnu).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timespec {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

/// `struct stat` (x86_64 linux-gnu).  144 bytes, `st_mtim` at offset 88.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: c_ulong,
    pub st_ino: c_ulong,
    pub st_nlink: c_ulong,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
    pub st_rdev: c_ulong,
    pub st_size: c_long,
    pub st_blksize: c_long,
    pub st_blocks: c_long,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [c_long; 3],
}

/// `struct tm` (x86_64 linux-gnu).  56 bytes.
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

pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;
pub const EXIT_FAILURE: c_int = 1;

unsafe extern "C" {
    pub static mut stderr: *mut FILE;

    pub fn calloc(num: usize, size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn exit(status: c_int) -> !;
    pub fn atoi(s: *const c_char) -> c_int;

    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strstr(hay: *const c_char, needle: *const c_char) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(fp: *mut FILE) -> c_int;
    pub fn fseek(fp: *mut FILE, off: c_long, whence: c_int) -> c_int;
    pub fn fgets(buf: *mut c_char, size: c_int, fp: *mut FILE) -> *mut c_char;
    pub fn feof(fp: *mut FILE) -> c_int;
    pub fn clearerr(fp: *mut FILE);
    pub fn fileno(fp: *mut FILE) -> c_int;
    pub fn perror(msg: *const c_char);
    pub fn fprintf(fp: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;

    pub fn fstat(fd: c_int, buf: *mut stat) -> c_int;
    pub fn select(
        nfds: c_int,
        readfds: *mut c_void,
        writefds: *mut c_void,
        exceptfds: *mut c_void,
        timeout: *mut timeval,
    ) -> c_int;

    pub fn __errno_location() -> *mut c_int;
}

/// `errno`
#[inline]
pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}

/// Emit a literal (already NUL-terminated) message through the C `stderr`
/// stream, exactly like `fprintf(stderr, "<literal>")`.
///
/// The literal contains no `%`, so passing it as the format string reproduces
/// the original byte stream verbatim.
#[inline]
pub fn fputs_stderr(msg: &'static [u8]) {
    debug_assert_eq!(*msg.last().unwrap(), 0);
    unsafe {
        fprintf(stderr, msg.as_ptr() as *const c_char);
    }
}

/// `os_free(x)` from `shared.h`: `if (x) { free(x); x = NULL; }`
#[inline]
pub unsafe fn os_free<T>(slot: *mut *mut T) {
    unsafe {
        if !(*slot).is_null() {
            free(*slot as *mut c_void);
            *slot = core::ptr::null_mut();
        }
    }
}

/// `os_clearnl(x, p)` from `shared.h`:
/// `if ((p = strrchr(x, '\n'))) *p = '\0';`
///
/// Returns the pointer the macro would have left in `p`.
#[inline]
pub unsafe fn os_clearnl(s: *mut c_char) -> *mut c_char {
    unsafe {
        let p = strrchr(s, b'\n' as c_int);
        if !p.is_null() {
            *p = 0;
        }
        p
    }
}
