//! Raw libc bindings and C-string helpers.
//!
//! The C library manipulates NUL terminated byte buffers with the classic
//! `<string.h>` primitives.  In order to guarantee byte-identical behaviour we
//! bind directly to libc for allocation / stdio (so that memory produced here
//! can be `free()`d by the caller, and so `FILE *` handles are interchangeable
//! with the ones the caller owns) and re-implement the pure scanning routines
//! as safe helpers over byte slices.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_uint, c_void};

/// Opaque `FILE`.
#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

pub type time_t = i64;
pub type off_t = i64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

/// glibc x86_64 `struct stat` (size 144, `st_mtim` at offset 88).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: c_uint,
    pub st_uid: c_uint,
    pub st_gid: c_uint,
    pub __pad0: c_int,
    pub st_rdev: u64,
    pub st_size: off_t,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [i64; 3],
}

/// glibc `struct tm`.
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

extern "C" {
    pub static mut stderr: *mut FILE;

    pub fn calloc(num: usize, size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    pub fn exit(status: c_int) -> !;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fgets(s: *mut c_char, size: c_int, stream: *mut FILE) -> *mut c_char;
    pub fn fseek(stream: *mut FILE, offset: c_long, whence: c_int) -> c_int;
    pub fn feof(stream: *mut FILE) -> c_int;
    pub fn clearerr(stream: *mut FILE);
    pub fn fileno(stream: *mut FILE) -> c_int;
    pub fn perror(s: *const c_char);

    pub fn fstat(fd: c_int, buf: *mut stat) -> c_int;
    pub fn select(
        nfds: c_int,
        readfds: *mut c_void,
        writefds: *mut c_void,
        exceptfds: *mut c_void,
        timeout: *mut timeval,
    ) -> c_int;

    pub fn __errno_location() -> *mut c_int;

    pub fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}

/// `errno`
#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

// ---------------------------------------------------------------------------
// Safe C-string helpers over a byte buffer.
//
// Every helper treats an out-of-range index as a NUL byte.  The buffers used by
// the translated code always have an explicit NUL terminator at their final
// index (mirroring `str[OS_MAXSTR] = '\0'`), so a genuine C traversal can never
// walk past the end of the array; the fallback only exists to keep these
// helpers total.
// ---------------------------------------------------------------------------

/// `strlen(&buf[start])`
#[inline]
pub fn c_len(buf: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < buf.len() && buf[i] != 0 {
        i += 1;
    }
    i - start
}

/// `strchr(&buf[start], ch)` -> index of the match.
///
/// Note that C's `strchr` can also match the terminating NUL; none of the call
/// sites in this library search for `'\0'`, so searching stops at the
/// terminator.
#[inline]
pub fn c_chr(buf: &[u8], start: usize, ch: u8) -> Option<usize> {
    let mut i = start;
    while i < buf.len() && buf[i] != 0 {
        if buf[i] == ch {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `strrchr(&buf[start], ch)` -> index of the last match.
#[inline]
pub fn c_rchr(buf: &[u8], start: usize, ch: u8) -> Option<usize> {
    let mut found = None;
    let mut i = start;
    while i < buf.len() && buf[i] != 0 {
        if buf[i] == ch {
            found = Some(i);
        }
        i += 1;
    }
    found
}

/// `strstr(&buf[start], needle)` -> index of the match. `needle` must not be
/// empty and must not contain a NUL.
pub fn c_str_find(buf: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    let mut i = start;
    loop {
        // Try to match `needle` at `i`.
        let mut k = 0usize;
        loop {
            if k == needle.len() {
                return Some(i);
            }
            let b = if i + k < buf.len() { buf[i + k] } else { 0 };
            if b != needle[k] {
                break;
            }
            k += 1;
        }
        // No match here; advance unless we are sitting on the terminator.
        let b = if i < buf.len() { buf[i] } else { 0 };
        if b == 0 {
            return None;
        }
        i += 1;
    }
}

/// `strncmp(lit, &buf[start], lit.len()) == 0`.
///
/// `lit` must not contain a NUL byte, which is true for every literal used by
/// the C sources; therefore the comparison is a plain prefix test with an
/// out-of-range byte behaving as a mismatching NUL.
#[inline]
pub fn c_ncmp_eq(buf: &[u8], start: usize, lit: &[u8]) -> bool {
    for (k, &want) in lit.iter().enumerate() {
        let got = match start.checked_add(k) {
            Some(i) if i < buf.len() => buf[i],
            _ => 0,
        };
        if got != want {
            return false;
        }
    }
    true
}

/// `atoi(&buf[start])`.
///
/// GCC lowers `atoi` to `strtol(p, NULL, 10)` truncated to `int` (this is
/// visible in the compiled C object, whose only relocation for these calls is
/// against `strtol`).  Delegate to libc so that overflow clamping matches.
#[inline]
pub unsafe fn c_atoi(buf: &[u8], start: usize) -> c_int {
    debug_assert!(start < buf.len());
    strtol(
        buf.as_ptr().add(start) as *const c_char,
        core::ptr::null_mut(),
        10,
    ) as c_int
}

// ---------------------------------------------------------------------------
// Raw-pointer helpers for heap allocated C strings.
// ---------------------------------------------------------------------------

/// `strlen(p)`
#[inline]
pub unsafe fn raw_len(p: *const c_char) -> usize {
    let mut n = 0usize;
    while *p.add(n) != 0 {
        n += 1;
    }
    n
}

/// `strrchr(p, ch)`
#[inline]
pub unsafe fn raw_rchr(p: *mut c_char, ch: u8) -> *mut c_char {
    let mut found: *mut c_char = core::ptr::null_mut();
    let mut i = 0usize;
    while *p.add(i) != 0 {
        if *p.add(i) as u8 == ch {
            found = p.add(i);
        }
        i += 1;
    }
    found
}

/// `strstr(p, needle) != NULL`. `needle` must be non-empty and NUL free.
pub unsafe fn raw_contains(p: *const c_char, needle: &[u8]) -> bool {
    let hay_len = raw_len(p);
    if needle.len() > hay_len {
        return false;
    }
    'outer: for i in 0..=(hay_len - needle.len()) {
        for (k, &want) in needle.iter().enumerate() {
            if *p.add(i + k) as u8 != want {
                continue 'outer;
            }
        }
        return true;
    }
    false
}

/// The `os_free(x)` macro: `if(x){free(x);x=NULL;};`
#[inline]
pub unsafe fn os_free_slot(slot: *mut *mut c_char) {
    if !(*slot).is_null() {
        free(*slot as *mut c_void);
        *slot = core::ptr::null_mut();
    }
}
