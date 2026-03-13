#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::os::raw::c_void;

extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strrchr(s: *const c_char, c: c_int) -> *const c_char;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strerror(errnum: c_int) -> *const c_char;
    fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    fn exit(status: c_int) -> !;

    static stderr: *mut c_void;
}

#[cfg(target_os = "linux")]
extern "C" {
    fn __errno_location() -> *mut c_int;
}

#[cfg(target_os = "linux")]
unsafe fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

#[cfg(target_os = "macos")]
extern "C" {
    fn __error() -> *mut c_int;
}

#[cfg(target_os = "macos")]
unsafe fn get_errno() -> c_int {
    unsafe { *__error() }
}

unsafe fn extract_filename(path: *const c_char, separator: c_char) -> *const c_char {
    unsafe {
        let search = strrchr(path, separator as c_int);
        if search.is_null() {
            return path;
        }
        search.add(1)
    }
}

/// # Safety
/// `path` and `outDirName` must be valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    unsafe {
        let separator: c_char = b'/' as c_char;

        let filename_start = extract_filename(path, separator);

        let out_len = strlen(outDirName);
        let fname_len = strlen(filename_start);
        let result = calloc(1, out_len + 1 + fname_len + suffixLen + 1) as *mut c_char;
        if result.is_null() {
            let fmt = b"zstd: FIO_createFilename_fromOutDir: %s\0".as_ptr() as *const c_char;
            fprintf(stderr, fmt, strerror(get_errno()));
            exit(30);
        }

        memcpy(result as *mut c_void, outDirName as *const c_void, out_len);
        if *outDirName.add(out_len - 1) == separator {
            memcpy(
                result.add(out_len) as *mut c_void,
                filename_start as *const c_void,
                fname_len,
            );
        } else {
            memcpy(
                result.add(out_len) as *mut c_void,
                &separator as *const c_char as *const c_void,
                1,
            );
            memcpy(
                result.add(out_len + 1) as *mut c_void,
                filename_start as *const c_void,
                fname_len,
            );
        }

        result
    }
}
