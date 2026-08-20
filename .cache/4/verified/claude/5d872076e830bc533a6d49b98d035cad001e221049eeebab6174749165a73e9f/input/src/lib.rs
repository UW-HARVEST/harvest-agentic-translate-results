//! Rust translation of `c_src/src/lib.c`.
//!
//! Exports the exact public ABI of the C shared library:
//!   * `extractFilename`
//!   * `FIO_createFilename_fromOutDir`
//!
//! Behaviour (including the out-of-bounds read on an empty `outDirName` and the
//! `strrchr` treatment of the terminating NUL byte) is reproduced verbatim; no
//! bugs from the original C are fixed.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings
//
// The returned buffer of `FIO_createFilename_fromOutDir` is documented (by the
// original zstd sources this file comes from) as being `free()`-able by the
// caller, so the allocation must go through libc's `calloc`, not Rust's
// allocator.
// ---------------------------------------------------------------------------
#[allow(non_camel_case_types)]
type FILE = c_void;

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn exit(status: c_int) -> !;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
    #[link_name = "stderr"]
    static mut c_stderr: *mut FILE;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
}

/// Platform path separator, mirroring the `#if defined(_MSC_VER) || ...` block.
/// This crate targets the same (non-Windows) configuration the C library was
/// built with, so the separator is `'/'`.
#[cfg(not(windows))]
const PATH_SEPARATOR: u8 = b'/';
#[cfg(windows)]
const PATH_SEPARATOR: u8 = b'\\';

// ---------------------------------------------------------------------------
// Small libc-string helpers, written so the exact C semantics are preserved.
// ---------------------------------------------------------------------------

/// `strlen()` equivalent.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut len: usize = 0;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

/// `strrchr()` equivalent.
///
/// Just like the C library function, the terminating NUL byte is considered
/// part of the string: searching for `'\0'` yields a pointer to the terminator.
unsafe fn c_strrchr(s: *const c_char, ch: c_char) -> *const c_char {
    let target = ch as u8;
    let mut found: *const c_char = std::ptr::null();
    let mut p = s;
    unsafe {
        loop {
            let b = *p as u8;
            if b == target {
                found = p;
            }
            if b == 0 {
                break;
            }
            p = p.add(1);
        }
    }
    found
}

/// `memcpy()` equivalent.
unsafe fn c_memcpy(dst: *mut c_char, src: *const c_char, n: usize) {
    unsafe {
        std::ptr::copy_nonoverlapping(src, dst, n);
    }
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// ```c
/// const char* extractFilename(const char* path, char separator)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    unsafe {
        let search = c_strrchr(path, separator);
        if search.is_null() {
            return path;
        }
        search.add(1)
    }
}

/// ```c
/// char* FIO_createFilename_fromOutDir(const char* path,
///                                     const char* outDirName,
///                                     const size_t suffixLen)
/// ```
///
/// Takes a source file name and specified output directory, and allocates
/// memory for and returns a pointer to final path.
/// This function never returns an error (it may `exit()` in case of pb).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    unsafe {
        let separator: c_char = PATH_SEPARATOR as c_char;

        let mut filenameStart: *const c_char = extractFilename(path, separator);
        if cfg!(windows) {
            /* sometimes, '/' separator is also used on Windows (mingw+msys2) */
            filenameStart = extractFilename(filenameStart, b'/' as c_char);
        }

        let outDirLen = c_strlen(outDirName);
        let filenameLen = c_strlen(filenameStart);

        /* size_t arithmetic: wraps, exactly as in C */
        let size = outDirLen
            .wrapping_add(1)
            .wrapping_add(filenameLen)
            .wrapping_add(suffixLen)
            .wrapping_add(1);

        let result = calloc(1, size) as *mut c_char;
        if result.is_null() {
            fprintf(
                c_stderr,
                c"zstd: FIO_createFilename_fromOutDir: %s".as_ptr(),
                strerror(*__errno_location()),
            );
            exit(30);
        }

        c_memcpy(result, outDirName, outDirLen);
        /* NOTE: when outDirName is the empty string this reads outDirName[-1],
         * an out-of-bounds access present in the original C code. It is
         * reproduced here on purpose. */
        if *outDirName.add(outDirLen.wrapping_sub(1)) as u8 == separator as u8 {
            c_memcpy(result.add(outDirLen), filenameStart, filenameLen);
        } else {
            c_memcpy(result.add(outDirLen), &separator as *const c_char, 1);
            c_memcpy(
                result.add(outDirLen.wrapping_add(1)),
                filenameStart,
                filenameLen,
            );
        }

        result
    }
}
