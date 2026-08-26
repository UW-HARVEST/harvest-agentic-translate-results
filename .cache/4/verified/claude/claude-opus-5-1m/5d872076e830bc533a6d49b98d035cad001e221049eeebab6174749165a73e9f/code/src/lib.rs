//! Rust translation of `c_src/src/lib.c`.
//!
//! Exports the exact public ABI of the C shared library:
//!   * `extractFilename`
//!   * `FIO_createFilename_fromOutDir`
//!
//! Behaviour (including the out-of-bounds read on an empty `outDirName`, the
//! `strrchr` treatment of the terminating NUL byte, the wrapping `size_t`
//! arithmetic of the allocation size and the `exit(30)` on allocation failure)
//! is reproduced verbatim; no bugs from the original C are fixed.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings
//
// The C translation unit calls libc for *everything* it does with memory and
// strings (`strlen`, `strrchr`, `memcpy`, `calloc`, `fprintf`, `strerror`,
// `exit`). This translation binds those very functions instead of
// reimplementing them, which keeps the observable behaviour identical down to
// the failure modes:
//
//   * the returned buffer comes from libc `calloc`, so a caller may `free()` it
//     exactly as with the C library;
//   * invalid arguments (e.g. a NULL `path`) fault inside the same libc routine
//     the C library would fault in, producing the same signal — a hand-written
//     Rust loop would instead trip rustc's debug-only null-pointer check and
//     abort with SIGABRT rather than SIGSEGV.
// ---------------------------------------------------------------------------
#[allow(non_camel_case_types)]
type FILE = c_void;

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn exit(status: c_int) -> !;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn __errno_location() -> *mut c_int;
    #[link_name = "stderr"]
    static mut c_stderr: *mut FILE;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
}

/// Platform path separator, mirroring the
/// `#if defined(_MSC_VER) || defined(__MINGW32__) || defined(__MSVCRT__)` block.
/// The C library in `c_src/` is built for the host (non-Windows), so the `#else`
/// branch applies and the separator is `'/'`.
#[cfg(not(windows))]
const PATH_SEPARATOR: u8 = b'/';
#[cfg(windows)]
const PATH_SEPARATOR: u8 = b'\\';

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// ```c
/// const char*
/// extractFilename(const char* path, char separator)
/// {
///     const char* search = strrchr(path, separator);
///     if (search == NULL) return path;
///     return search+1;
/// }
/// ```
///
/// Note that `separator == '\0'` is *found* by `strrchr` (the terminating NUL is
/// part of the searched string), so the result is then one past the terminator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    unsafe {
        // `separator` is a `char` in C and is promoted to `int` for the call to
        // `strrchr`, which converts it back with `(char)c`. Reproducing the
        // promotion of the (signed, on this target) `c_char` keeps every one of
        // the 256 byte values behaving identically.
        let search = strrchr(path, separator as c_int);
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

        #[allow(unused_mut)]
        let mut filenameStart: *const c_char = extractFilename(path, separator);
        #[cfg(windows)]
        {
            /* sometimes, '/' separator is also used on Windows (mingw+msys2) */
            filenameStart = extractFilename(filenameStart, b'/' as c_char);
        }

        let outDirLen = strlen(outDirName);
        let filenameLen = strlen(filenameStart);

        /* `size_t` arithmetic: wraps on overflow, exactly as in C */
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

        memcpy(result as *mut c_void, outDirName as *const c_void, outDirLen);

        /* NOTE: when `outDirName` is the empty string, `strlen(outDirName)-1`
         * wraps to SIZE_MAX and this reads the byte *before* the buffer — an
         * out-of-bounds read present in the original C code, reproduced here on
         * purpose (`wrapping_add` computes the same address the C does). */
        let lastByte = outDirName.wrapping_add(outDirLen.wrapping_sub(1));
        if *lastByte == separator {
            memcpy(
                result.add(outDirLen) as *mut c_void,
                filenameStart as *const c_void,
                filenameLen,
            );
        } else {
            memcpy(
                result.add(outDirLen) as *mut c_void,
                &separator as *const c_char as *const c_void,
                1,
            );
            memcpy(
                result.add(outDirLen.wrapping_add(1)) as *mut c_void,
                filenameStart as *const c_void,
                filenameLen,
            );
        }

        result
    }
}
