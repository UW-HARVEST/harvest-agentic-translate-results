use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn exit(status: c_int) -> !;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
    fn strerror(error_number: c_int) -> *mut c_char;
    fn strlen(string: *const c_char) -> usize;
    fn strrchr(string: *const c_char, character: c_int) -> *mut c_char;

    static mut stderr: *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = unsafe { strrchr(path, separator as c_int) };
    if search.is_null() {
        path
    } else {
        unsafe { search.add(1) }.cast_const()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    const FORMAT: &[u8] = b"zstd: FIO_createFilename_fromOutDir: %s\0";
    let separator = b'/' as c_char;
    let filename_start = unsafe { extractFilename(path, separator) };

    let allocation_size = unsafe { strlen(out_dir_name) }
        .wrapping_add(1)
        .wrapping_add(unsafe { strlen(filename_start) })
        .wrapping_add(suffix_len)
        .wrapping_add(1);
    let result = unsafe { calloc(1, allocation_size) }.cast::<c_char>();

    if result.is_null() {
        let error_number = unsafe { *__errno_location() };
        unsafe {
            fprintf(stderr, FORMAT.as_ptr().cast(), strerror(error_number));
            exit(30);
        }
    }

    let out_dir_len = unsafe { strlen(out_dir_name) };
    unsafe {
        memcpy(result.cast(), out_dir_name.cast(), out_dir_len);
    }

    if unsafe { *out_dir_name.wrapping_add(out_dir_len.wrapping_sub(1)) == separator } {
        let filename_len = unsafe { strlen(filename_start) };
        unsafe {
            memcpy(
                result.add(out_dir_len).cast(),
                filename_start.cast(),
                filename_len,
            );
        }
    } else {
        unsafe {
            memcpy(
                result.add(out_dir_len).cast(),
                ptr::from_ref(&separator).cast(),
                1,
            );
        }
        let filename_len = unsafe { strlen(filename_start) };
        unsafe {
            memcpy(
                result.add(out_dir_len + 1).cast(),
                filename_start.cast(),
                filename_len,
            );
        }
    }

    result
}
