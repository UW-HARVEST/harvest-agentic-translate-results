use std::ffi::{c_char, c_int};

use libc::FILE;

extern "C" {
    static stderr: *mut FILE;
}

unsafe fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        libc::fprintf(stderr, b"Error: negative input\n\0".as_ptr().cast());
        return -1;
    }

    libc::printf(b"Processing: %d\n\0".as_ptr().cast(), x);
    x * 2
}

unsafe fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    let fp = libc::fopen(filename, b"r\0".as_ptr().cast());
    if fp.is_null() {
        libc::fprintf(
            stderr,
            b"Error: opening or processing file %s\n\0".as_ptr().cast(),
            filename,
        );
        return std::ptr::null_mut();
    }

    let mut buffer = [0u8; 100];
    while !libc::fgets(buffer.as_mut_ptr().cast(), 100, fp).is_null() {
        libc::printf(b"%s\0".as_ptr().cast(), buffer.as_ptr());
    }

    if libc::ferror(fp) != 0 {
        libc::fprintf(
            stderr,
            b"Error: opening or processing file %s\n\0".as_ptr().cast(),
            filename,
        );
        libc::fclose(fp);
        return std::ptr::null_mut();
    }

    fp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        libc::printf(b"Goto output: %d\n\0".as_ptr().cast(), res);
    }

    let out = open_with_cleanup(filename);
    if out.is_null() {
        return -2;
    } else {
        libc::fclose(out);
    }

    0
}
