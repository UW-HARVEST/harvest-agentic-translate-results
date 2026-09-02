//! Rust translation of `c_src/src/lib.c` (zstd's `FIO_createFilename_fromOutDir`
//! helper and its `extractFilename` companion).
//!
//! The translation is deliberately literal: allocation is still performed with
//! libc `calloc` (so callers may `free()` the result exactly as with the C
//! library), the order of operations and error handling is preserved, and the
//! original out-of-bounds read on an empty `outDirName` is reproduced rather
//! than "fixed".
//!
//! Exported ABI (must match `nm -D` of the C shared object):
//!   * `extractFilename`
//!   * `FIO_createFilename_fromOutDir`

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

// Minimal libc bindings. Using the real libc entry points keeps the observable
// behaviour (including corner cases such as `strrchr` with a NUL separator, and
// `calloc`-owned memory that the caller frees) identical to the C build.
unsafe extern "C" {
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn fputs(s: *const c_char, stream: *mut c_void) -> c_int;
    fn exit(status: c_int) -> !;
    fn __errno_location() -> *mut c_int;

    // glibc exports `stderr` as a data symbol holding a `FILE*`.
    static stderr: *mut c_void;
}

/// The path separator selected by the C preprocessor. The C sources pick `'\\'`
/// on MSVC/MinGW/MSVCRT builds and `'/'` everywhere else; the reference build
/// (CMake on Linux) takes the `'/'` branch, and so does this translation.
const SEPARATOR: c_char = b'/' as c_char;

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
/// Note that `separator` is widened to `int` by the implicit conversion at the
/// `strrchr` call site, matching the C code; a `separator` of `'\0'` therefore
/// finds the terminating NUL, and the function returns a pointer just past the
/// end of the string. That behaviour is reproduced verbatim.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(
    path: *const c_char,
    separator: c_char,
) -> *const c_char {
    unsafe {
        let search = strrchr(path, separator as c_int);
        if search.is_null() {
            return path;
        }
        search.add(1)
    }
}

/// ```c
/// char*
/// FIO_createFilename_fromOutDir(const char* path, const char* outDirName,
///                               const size_t suffixLen)
/// ```
///
/// Takes a source file name and specified output directory, and allocates
/// memory for and returns a pointer to the final path. This function never
/// returns an error (it may `exit(30)` in case of allocation failure).
///
/// The returned buffer comes from `calloc`, so the trailing `suffixLen + 1`
/// bytes are guaranteed zero, byte-for-byte as in the C original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    unsafe {
        let separator: c_char = SEPARATOR;

        let filename_start = extractFilename(path, separator);
        // The Windows-only second `extractFilename(filenameStart, '/')` pass is
        // compiled out on this platform, so it is intentionally absent here.

        let out_dir_len = strlen(outDirName);
        let filename_len = strlen(filename_start);

        let result = calloc(1, out_dir_len + 1 + filename_len + suffixLen + 1) as *mut c_char;
        if result.is_null() {
            // fprintf(stderr, "zstd: FIO_createFilename_fromOutDir: %s", strerror(errno));
            // Two `fputs` calls emit exactly the same byte sequence as the
            // single `fprintf` with a trailing "%s" conversion.
            fputs(
                c"zstd: FIO_createFilename_fromOutDir: ".as_ptr(),
                stderr,
            );
            fputs(strerror(*__errno_location()), stderr);
            exit(30);
        }

        memcpy(result as *mut c_void, outDirName as *const c_void, out_dir_len);

        // Faithful reproduction of `outDirName[strlen(outDirName)-1]`: when
        // `outDirName` is the empty string this reads one byte *before* the
        // buffer. The C code does the same, so the behaviour is preserved
        // instead of guarded against.
        let last_byte = *outDirName.offset(out_dir_len.wrapping_sub(1) as isize);

        if last_byte == separator {
            memcpy(
                result.add(out_dir_len) as *mut c_void,
                filename_start as *const c_void,
                filename_len,
            );
        } else {
            memcpy(
                result.add(out_dir_len) as *mut c_void,
                (&raw const separator) as *const c_void,
                1,
            );
            memcpy(
                result.add(out_dir_len + 1) as *mut c_void,
                filename_start as *const c_void,
                filename_len,
            );
        }

        result
    }
}
