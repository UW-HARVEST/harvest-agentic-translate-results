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

unsafe fn extract_filename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = unsafe { strrchr(path, separator as c_int) };
    if search.is_null() {
        return path;
    }
    unsafe { search.add(1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    let separator: c_char = b'/' as c_char;

    let filename_start = unsafe { extract_filename(path, separator) };

    let out_dir_len = unsafe { strlen(out_dir_name) };
    let filename_len = unsafe { strlen(filename_start) };

    let result = unsafe { calloc(1, out_dir_len + 1 + filename_len + suffix_len + 1) } as *mut c_char;
    if result.is_null() {
        unsafe {
            let errno_ptr = libc_errno();
            fprintf(
                stderr,
                b"zstd: FIO_createFilename_fromOutDir: %s\0".as_ptr() as *const c_char,
                strerror(errno_ptr),
            );
            exit(30);
        }
    }

    unsafe {
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
    }

    result
}

/// Get errno value via C runtime
fn libc_errno() -> c_int {
    // errno is thread-local; access via __errno_location on Linux
    extern "C" {
        fn __errno_location() -> *mut c_int;
    }
    unsafe { *__errno_location() }
}
