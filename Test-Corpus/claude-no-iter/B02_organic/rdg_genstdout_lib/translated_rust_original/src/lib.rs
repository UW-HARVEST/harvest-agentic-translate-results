use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;

extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strrchr(s: *const c_char, c: c_int) -> *const c_char;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn strerror(errnum: c_int) -> *const c_char;
    fn exit(status: c_int) -> !;

    // errno is typically a thread-local; access via __errno_location on glibc.
    fn __errno_location() -> *mut c_int;
}

// Provide a way to obtain stderr that matches C's `stderr` global.
// On Linux glibc, `stderr` is an exported symbol of type FILE*.
extern "C" {
    static mut stderr: *mut c_void;
}

unsafe fn extract_filename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = strrchr(path, separator as c_int);
    if search.is_null() {
        return path;
    }
    search.add(1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    // Non-Windows build: separator is '/'.
    let separator: c_char = b'/' as c_char;

    let filename_start = extract_filename(path, separator);

    let out_dir_len = strlen(out_dir_name);
    let filename_len = strlen(filename_start);

    let total = out_dir_len + 1 + filename_len + suffix_len + 1;
    let result = calloc(1, total) as *mut c_char;
    if result.is_null() {
        let fmt = b"zstd: FIO_createFilename_fromOutDir: %s\0".as_ptr() as *const c_char;
        let err_msg = strerror(*__errno_location());
        fprintf(stderr, fmt, err_msg);
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
        memcpy(
            result.add(out_dir_len) as *mut c_void,
            &separator as *const c_char as *const c_void,
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
