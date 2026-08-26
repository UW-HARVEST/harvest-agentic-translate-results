use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    static mut stderr: *mut c_void;

    fn __errno_location() -> *mut c_int;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn exit(status: c_int) -> !;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
    fn strerror(error_number: c_int) -> *mut c_char;
    fn strlen(value: *const c_char) -> usize;
    fn strrchr(value: *const c_char, character: c_int) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = unsafe { strrchr(path, c_int::from(separator)) };
    if search.is_null() {
        path
    } else {
        unsafe { search.add(1) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    let separator = b'/' as c_char;
    let filename_start = unsafe { extractFilename(path, separator) };

    let allocation_size = unsafe { strlen(out_dir_name) }
        .wrapping_add(1)
        .wrapping_add(unsafe { strlen(filename_start) })
        .wrapping_add(suffix_len)
        .wrapping_add(1);
    let result = unsafe { calloc(1, allocation_size) }.cast::<c_char>();

    if result.is_null() {
        const MESSAGE: &[u8] = b"zstd: FIO_createFilename_fromOutDir: %s\0";
        let error_number = unsafe { *__errno_location() };
        unsafe {
            fprintf(
                stderr,
                MESSAGE.as_ptr().cast::<c_char>(),
                strerror(error_number),
            );
            exit(30);
        }
    }

    let out_dir_len = unsafe { strlen(out_dir_name) };
    unsafe {
        memcpy(
            result.cast::<c_void>(),
            out_dir_name.cast::<c_void>(),
            out_dir_len,
        );
    }

    if unsafe { *out_dir_name.add(strlen(out_dir_name).wrapping_sub(1)) } == separator {
        unsafe {
            memcpy(
                result.add(strlen(out_dir_name)).cast::<c_void>(),
                filename_start.cast::<c_void>(),
                strlen(filename_start),
            );
        }
    } else {
        unsafe {
            memcpy(
                result.add(strlen(out_dir_name)).cast::<c_void>(),
                (&separator as *const c_char).cast::<c_void>(),
                1,
            );
            memcpy(
                result
                    .add(strlen(out_dir_name).wrapping_add(1))
                    .cast::<c_void>(),
                filename_start.cast::<c_void>(),
                strlen(filename_start),
            );
        }
    }

    result
}
