//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C shared object):
//!   * `extractFilename`
//!   * `FIO_createFilename_fromOutDir`
//!
//! The behaviour of the C original is reproduced exactly, including its quirks
//! (e.g. the out-of-bounds `outDirName[strlen(outDirName)-1]` read when
//! `outDirName` is the empty string, and the `exit(30)` on allocation failure).

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings
//
// The buffer returned by FIO_createFilename_fromOutDir() is expected to be
// released by the caller with free(), therefore it must come from the very same
// allocator the C code used: libc's calloc().
// ---------------------------------------------------------------------------
// The string routines are imported from libc as well, rather than
// re-implemented, so that every observable detail matches the C original
// bit-for-bit -- including what happens for invalid inputs. A hand-written Rust
// `strlen` would, under `-C debug-assertions`, trip Rust's raw-pointer UB checks
// and `abort()` (SIGABRT) where the C faults (SIGSEGV); calling the very same
// libc symbols the C calls removes that whole class of divergence.
unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn exit(status: c_int) -> !;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;

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
    // SAFETY: `path` is a NUL terminated C string, as required by the C API.
    // The `char` argument is promoted to `int` exactly as the C compiler does
    // for the `strrchr(path, separator)` call (sign-extending on platforms with
    // a signed `char`).
    let search: *const c_char = unsafe { strrchr(path, separator as c_int) };
    if search.is_null() {
        return path;
    }
    // `search+1` mirrors the C literally; `wrapping_add` avoids imposing any
    // Rust-only provenance/bounds requirement that the C does not have (e.g.
    // `separator == 0`, where `search` points at the terminator and the result
    // is deliberately one past the end of the string).
    search.wrapping_add(1)
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
                result.wrapping_add(outDirLen) as *mut c_void,
                filenameStart as *const c_void,
                filenameLen,
            );
        } else {
            memcpy(
                result.wrapping_add(outDirLen) as *mut c_void,
                &separator as *const c_char as *const c_void,
                1,
            );
            memcpy(
                result.wrapping_add(outDirLen).wrapping_add(1) as *mut c_void,
                filenameStart as *const c_void,
                filenameLen,
            );
        }

        result
    }
}
