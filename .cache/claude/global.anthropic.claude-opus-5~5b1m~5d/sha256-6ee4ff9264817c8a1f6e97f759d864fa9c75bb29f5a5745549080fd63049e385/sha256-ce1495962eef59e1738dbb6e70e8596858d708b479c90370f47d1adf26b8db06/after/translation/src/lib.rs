//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C shared object):
//!   * `extractFilename`
//!   * `FIO_createFilename_fromOutDir`
//!
//! The behaviour of the C original is reproduced exactly, including its quirks
//! (e.g. the out-of-bounds `outDirName[strlen(outDirName)-1]` read when
//! `outDirName` is the empty string, and the `exit(30)` on allocation failure),
//! and including the fatal signal it raises on a NULL argument — the C performs
//! no null checks whatsoever, so neither does this translation.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings
//
// The buffer returned by FIO_createFilename_fromOutDir() is expected to be
// released by the caller with free(), therefore it must come from the very same
// allocator the C code used: libc's calloc().
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn exit(status: c_int) -> !;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;

    // The C source calls these libc routines directly, so the translation binds
    // to the very same implementations rather than re-implementing them. This
    // matters for more than just speed: a hand-written Rust `strlen` triggers
    // rustc's "null pointer dereference occurred" check, which turns into a
    // non-unwinding panic (SIGABRT) inside an `extern "C"` function, whereas the
    // C faults inside glibc with SIGSEGV. Binding to libc reproduces the C's
    // fatal-signal behaviour exactly, under every build profile.
    fn strlen(s: *const c_char) -> usize;
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    #[allow(non_upper_case_globals)]
    static mut stderr: *mut c_void;
}

/// Platform path separator, mirroring the `#if defined(_MSC_VER) ||
/// defined(__MINGW32__) || defined(__MSVCRT__)` block of the C source.
#[cfg(windows)]
const SEPARATOR: c_char = b'\\' as c_char;
#[cfg(not(windows))]
const SEPARATOR: c_char = b'/' as c_char;

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// ```c
/// const char* extractFilename(const char* path, char separator)
/// ```
///
/// Returns the portion of `path` following the last occurrence of `separator`,
/// or `path` itself when the separator does not occur.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    // `strrchr(path, separator)`: the C `char` argument undergoes the usual
    // integer promotion to `int`, i.e. it is SIGN-extended (`char` is signed on
    // this target). `separator as c_int` reproduces that promotion exactly.
    // SAFETY: `path` is a NUL terminated C string, as required by the C API.
    let search: *const c_char = unsafe { strrchr(path, separator as c_int) };
    if search.is_null() {
        return path;
    }
    // SAFETY: `search` points inside `path`, so one-past it is in bounds.
    unsafe { search.add(1) }
}

/// ```c
/// char* FIO_createFilename_fromOutDir(const char* path,
///                                     const char* outDirName,
///                                     const size_t suffixLen)
/// ```
///
/// Takes a source file name and specified output directory, and allocates
/// memory for and returns a pointer to the final path. This function never
/// returns an error (it may `exit(30)` in case of a problem).
///
/// The returned buffer is obtained from libc `calloc()` and must be released by
/// the caller with `free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    let separator: c_char = SEPARATOR;

    // SAFETY: both pointers are NUL terminated C strings, as required by the C
    // API; all pointer arithmetic below mirrors the original C exactly.
    unsafe {
        let mut filenameStart: *const c_char = extractFilename(path, separator);
        if cfg!(windows) {
            /* sometimes, '/' separator is also used on Windows (mingw+msys2) */
            filenameStart = extractFilename(filenameStart, b'/' as c_char);
        }

        let outDirLen = strlen(outDirName);
        let filenameLen = strlen(filenameStart);

        // calloc(1, strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1)
        // Wrapping arithmetic reproduces the C size_t overflow behaviour.
        let size = outDirLen
            .wrapping_add(1)
            .wrapping_add(filenameLen)
            .wrapping_add(suffixLen)
            .wrapping_add(1);
        let result = calloc(1, size) as *mut c_char;
        if result.is_null() {
            fprintf(
                stderr,
                c"zstd: FIO_createFilename_fromOutDir: %s".as_ptr(),
                strerror(*__errno_location()),
            );
            exit(30);
        }

        memcpy(result as *mut c_void, outDirName as *const c_void, outDirLen);
        // NOTE: for an empty `outDirName` the C code reads `outDirName[-1]`;
        // that out-of-bounds access is preserved verbatim here.
        if *outDirName.wrapping_add(outDirLen.wrapping_sub(1)) == separator {
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
                result.add(outDirLen).add(1) as *mut c_void,
                filenameStart as *const c_void,
                filenameLen,
            );
        }

        result
    }
}
