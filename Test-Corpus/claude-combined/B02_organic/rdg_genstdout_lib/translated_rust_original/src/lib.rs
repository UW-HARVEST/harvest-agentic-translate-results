#![allow(non_snake_case)]

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn strrchr(s: *const c_char, c: c_int) -> *const c_char;
    fn strlen(s: *const c_char) -> usize;
    fn calloc(nmemb: usize, size: usize) -> *mut std::ffi::c_void;
    fn memcpy(
        dest: *mut std::ffi::c_void,
        src: *const std::ffi::c_void,
        n: usize,
    ) -> *mut std::ffi::c_void;
    fn strerror(errnum: c_int) -> *const c_char;
    fn fprintf(stream: *mut std::ffi::c_void, fmt: *const c_char, ...) -> c_int;
    fn exit(status: c_int) -> !;

    static stderr: *mut std::ffi::c_void;
}

// Linux/glibc errno location
extern "C" {
    fn __errno_location() -> *mut c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(
    path: *const c_char,
    separator: c_char,
) -> *const c_char {
    let search = strrchr(path, separator as c_int);
    if search.is_null() {
        return path;
    }
    search.add(1)
}

/// FIO_createFilename_fromOutDir():
/// Takes a source file name and specified output directory, and
/// allocates memory for and returns a pointer to final path.
/// This function never returns an error (it may abort() in case of pb)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    // Non-windows separator
    let separator: c_char = b'/' as c_char;

    let filename_start = extractFilename(path, separator);

    let out_dir_len = strlen(outDirName);
    let filename_len = strlen(filename_start);

    let total_size = out_dir_len + 1 + filename_len + suffixLen + 1;
    let result = calloc(1, total_size) as *mut c_char;
    if result.is_null() {
        let errno = *__errno_location();
        let fmt = b"zstd: FIO_createFilename_fromOutDir: %s\0";
        fprintf(stderr, fmt.as_ptr() as *const c_char, strerror(errno));
        exit(30);
    }

    memcpy(
        result as *mut std::ffi::c_void,
        outDirName as *const std::ffi::c_void,
        out_dir_len,
    );

    // Check last char of outDirName
    let last_char = *outDirName.add(out_dir_len - 1);
    if last_char == separator {
        memcpy(
            result.add(out_dir_len) as *mut std::ffi::c_void,
            filename_start as *const std::ffi::c_void,
            filename_len,
        );
    } else {
        let sep_ptr: *const c_char = &separator;
        memcpy(
            result.add(out_dir_len) as *mut std::ffi::c_void,
            sep_ptr as *const std::ffi::c_void,
            1,
        );
        memcpy(
            result.add(out_dir_len + 1) as *mut std::ffi::c_void,
            filename_start as *const std::ffi::c_void,
            filename_len,
        );
    }

    result
}
