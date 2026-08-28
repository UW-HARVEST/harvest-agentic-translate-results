//! Rust translation of `c_src/src/lib.c` (zstd's `FIO_createFilename_fromOutDir` helper).
//!
//! The behaviour of the original C is reproduced exactly, including its
//! out-of-bounds read when `outDirName` is the empty string.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::ptr;

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
}

/// Path separator selected by the C preprocessor. The Windows branches of the
/// original `#if defined(_MSC_VER) || defined(__MINGW32__) || defined(__MSVCRT__)`
/// are only taken on Windows targets.
#[cfg(windows)]
const SEPARATOR: c_char = b'\\' as c_char;
#[cfg(not(windows))]
const SEPARATOR: c_char = b'/' as c_char;

/// `strlen(3)`
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut len = 0usize;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    len
}

/// `strrchr(3)`: the terminating NUL is part of the searched string, so
/// `c == 0` yields a pointer to it.
unsafe fn c_strrchr(s: *const c_char, c: c_char) -> *const c_char {
    let mut last: *const c_char = ptr::null();
    let mut p = s;
    loop {
        let ch = unsafe { *p };
        if ch == c {
            last = p;
        }
        if ch == 0 {
            break;
        }
        p = unsafe { p.add(1) };
    }
    last
}

/// `memcpy(3)`
unsafe fn c_memcpy(dst: *mut c_char, src: *const c_char, n: usize) {
    if n != 0 {
        unsafe { ptr::copy_nonoverlapping(src, dst, n) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = unsafe { c_strrchr(path, separator) };
    if search.is_null() {
        return path;
    }
    unsafe { search.add(1) }
}

/* FIO_createFilename_fromOutDir() :
 * Takes a source file name and specified output directory, and
 * allocates memory for and returns a pointer to final path.
 * This function never returns an error (it may abort() in case of pb)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    let separator: c_char = SEPARATOR;

    #[allow(unused_mut)]
    let mut filename_start = unsafe { extractFilename(path, separator) };
    #[cfg(windows)]
    {
        /* sometimes, '/' separator is also used on Windows (mingw+msys2) */
        filename_start = unsafe { extractFilename(filename_start, b'/' as c_char) };
    }

    let out_dir_len = unsafe { c_strlen(outDirName) };
    let filename_len = unsafe { c_strlen(filename_start) };

    let result =
        unsafe { calloc(1, out_dir_len + 1 + filename_len + suffixLen + 1) } as *mut c_char;
    if result.is_null() {
        let errno = unsafe { *__errno_location() };
        let msg = unsafe { strerror(errno) };
        let mut out = Vec::new();
        out.extend_from_slice(b"zstd: FIO_createFilename_fromOutDir: ");
        if !msg.is_null() {
            let msg_len = unsafe { c_strlen(msg) };
            out.extend_from_slice(unsafe {
                std::slice::from_raw_parts(msg as *const u8, msg_len)
            });
        }
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(&out);
        let _ = stderr.flush();
        std::process::exit(30);
    }

    unsafe {
        c_memcpy(result, outDirName, out_dir_len);
        /* NOTE: reproduces the original C, which reads outDirName[-1] when
         * outDirName is the empty string. */
        if *outDirName.add(out_dir_len).offset(-1) == separator {
            c_memcpy(result.add(out_dir_len), filename_start, filename_len);
        } else {
            c_memcpy(result.add(out_dir_len), &separator, 1);
            c_memcpy(result.add(out_dir_len + 1), filename_start, filename_len);
        }
    }

    result
}
