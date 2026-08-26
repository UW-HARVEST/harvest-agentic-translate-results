use std::ffi::{c_char, c_int};
use std::os::raw::c_void;

extern "C" {
    fn strrchr(s: *const c_char, c: c_int) -> *const c_char;
    fn strlen(s: *const c_char) -> usize;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strerror(errnum: c_int) -> *const c_char;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn exit(status: c_int) -> !;

    static stderr: *mut c_void;
}

// Matches: errno from libc
extern "C" {
    fn __errno_location() -> *mut c_int;
}

unsafe fn errno() -> c_int {
    unsafe { *__errno_location() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    unsafe {
        let search = strrchr(path, separator as c_int);
        if search.is_null() {
            path
        } else {
            search.add(1)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    unsafe {
        let separator: c_char = b'/' as c_char;

        let filename_start = extractFilename(path, separator);

        let out_dir_len = strlen(out_dir_name);
        let filename_len = strlen(filename_start);
        let result = calloc(1, out_dir_len + 1 + filename_len + suffix_len + 1) as *mut c_char;
        if result.is_null() {
            fprintf(
                stderr,
                b"zstd: FIO_createFilename_fromOutDir: %s\0".as_ptr() as *const c_char,
                strerror(errno()),
            );
            exit(30);
        }

        memcpy(
            result as *mut c_void,
            out_dir_name as *const c_void,
            out_dir_len,
        );

        if *out_dir_name.add(out_dir_len - 1) == separator {
            memcpy(
                result.add(out_dir_len) as *mut c_void,
                filename_start as *const c_void,
                filename_len,
            );
        } else {
            *result.add(out_dir_len) = separator;
            memcpy(
                result.add(out_dir_len + 1) as *mut c_void,
                filename_start as *const c_void,
                filename_len,
            );
        }

        result
    }
}
