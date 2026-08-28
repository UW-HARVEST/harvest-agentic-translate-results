//! Minimal bindings to the pieces of the C runtime that the original program
//! relies on, plus faithful re-implementations of the libc helpers whose exact
//! behaviour is observable (`atoi`, `strtok_r`, `perror`).
//!
//! Heap blocks handed out to / freed by callers must come from the same
//! allocator the C version used, because callers still call `free()` on them
//! (`driver()` itself does). Hence `malloc`/`free` rather than Rust's allocator.

use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;

#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut u8;
    pub fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    pub fn free(ptr: *mut u8);

    pub fn strerror(errnum: c_int) -> *const c_char;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fputs(s: *const c_char, stream: *mut FILE) -> c_int;
    pub fn fclose(stream: *mut FILE) -> c_int;
}

/// `EINVAL` on Linux.
pub const EINVAL: c_int = 22;

/// `EXIT_SUCCESS` / `EXIT_FAILURE` from `<stdlib.h>`.
pub const EXIT_SUCCESS: c_int = 0;
pub const EXIT_FAILURE: c_int = 1;

/// Current value of `errno`.
pub fn errno() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// `strerror(errnum)` as a byte string (no trailing NUL).
pub fn strerror_bytes(errnum: c_int) -> Vec<u8> {
    let mut out = Vec::new();
    unsafe {
        let mut p = strerror(errnum);
        if !p.is_null() {
            while *p != 0 {
                out.push(*p as u8);
                p = p.add(1);
            }
        }
    }
    out
}

/// Write raw bytes to stderr. C's `stderr` is unbuffered and so is Rust's, so
/// this is byte-for-byte equivalent to `fprintf(stderr, ...)`.
pub fn stderr_write(bytes: &[u8]) {
    let mut err = std::io::stderr();
    let _ = err.write_all(bytes);
    let _ = err.flush();
}

/// `perror(msg)`: `"<msg>: <strerror(errno)>\n"` on stderr.
pub fn perror(msg: &str) {
    let mut buf = Vec::with_capacity(msg.len() + 32);
    buf.extend_from_slice(msg.as_bytes());
    buf.extend_from_slice(b": ");
    buf.extend_from_slice(&strerror_bytes(errno()));
    buf.push(b'\n');
    stderr_write(&buf);
}

/// C's `int`-to-`size_t` conversion: sign-extend, then reinterpret.
#[inline]
pub fn int_to_size_t(v: c_int) -> usize {
    v as isize as usize
}

/// `strlen`.
///
/// # Safety
/// `s` must point to a NUL-terminated buffer.
pub unsafe fn strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// `strdup`: `malloc` a copy of the NUL-terminated string at `s`.
///
/// # Safety
/// `s` must point to a NUL-terminated buffer.
pub unsafe fn strdup(s: *const c_char) -> *mut c_char {
    unsafe {
        let len = strlen(s);
        let buf = malloc(len + 1);
        if buf.is_null() {
            return std::ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(s as *const u8, buf, len + 1);
        buf as *mut c_char
    }
}

/// `strtok_r` restricted to a single delimiter byte, which is all the original
/// code uses (`"\n"` and `" "`). Semantics match glibc: leading delimiters are
/// skipped, the delimiter that terminates the token is overwritten with NUL,
/// and NULL is returned once nothing but delimiters remain.
///
/// # Safety
/// `s` (when non-null) and `*saveptr` must point into a writable
/// NUL-terminated buffer.
pub unsafe fn strtok_r(s: *mut c_char, delim: u8, saveptr: &mut *mut c_char) -> *mut c_char {
    unsafe {
        let mut cur = if s.is_null() { *saveptr } else { s };

        // Skip leading delimiters.
        while *cur != 0 && *cur as u8 == delim {
            cur = cur.add(1);
        }

        if *cur == 0 {
            *saveptr = cur;
            return std::ptr::null_mut();
        }

        let token = cur;

        // Find the end of the token.
        while *cur != 0 && *cur as u8 != delim {
            cur = cur.add(1);
        }

        if *cur == 0 {
            *saveptr = cur;
        } else {
            *cur = 0;
            *saveptr = cur.add(1);
        }

        token
    }
}

#[inline]
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `atoi`, matching glibc: it is `(int) strtol(s, NULL, 10)`, so out-of-range
/// input saturates at `LONG_MAX`/`LONG_MIN` and is then truncated to `int`.
///
/// # Safety
/// `s` must point to a NUL-terminated buffer.
pub unsafe fn atoi(s: *const c_char) -> c_int {
    unsafe {
        let mut p = s;

        while is_c_space(*p as u8) {
            p = p.add(1);
        }

        let mut negative = false;
        if *p as u8 == b'-' {
            negative = true;
            p = p.add(1);
        } else if *p as u8 == b'+' {
            p = p.add(1);
        }

        // Accumulate as unsigned magnitude, saturating like strtol does.
        let limit: u64 = if negative {
            i64::MIN.unsigned_abs()
        } else {
            i64::MAX as u64
        };

        let mut acc: u64 = 0;
        let mut overflow = false;
        while (*p as u8).is_ascii_digit() {
            let digit = ((*p as u8) - b'0') as u64;
            if !overflow {
                match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(v) if v <= limit => acc = v,
                    _ => {
                        overflow = true;
                        acc = limit;
                    }
                }
            }
            p = p.add(1);
        }

        let as_long: i64 = if negative {
            (acc as i64).wrapping_neg()
        } else {
            acc as i64
        };

        as_long as c_int
    }
}
