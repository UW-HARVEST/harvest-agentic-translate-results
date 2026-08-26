use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc_FILE, fmt: *const c_char, ...) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut libc_FILE;
    fn fclose(fp: *mut libc_FILE) -> c_int;
    fn fgets(buf: *mut c_char, size: c_int, stream: *mut libc_FILE) -> *mut c_char;
    fn ferror(stream: *mut libc_FILE) -> c_int;
    static stderr: *mut libc_FILE;
}

#[repr(C)]
pub struct libc_FILE {
    _opaque: [u8; 0],
}

#[unsafe(no_mangle)]
pub extern "C" fn forward_goto_example(x: c_int) -> c_int {
    unsafe {
        if x < 0 {
            // goto error
            fprintf(stderr, b"Error: negative input\n\0".as_ptr() as *const c_char);
            return -1;
        }

        printf(
            b"Processing: %d\n\0".as_ptr() as *const c_char,
            x,
        );
        x * 2
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut libc_FILE {
    unsafe {
        let fp = fopen(filename, b"r\0".as_ptr() as *const c_char);
        if fp.is_null() {
            // goto cleanup (fp is null, so no fclose)
            fprintf(
                stderr,
                b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
                filename,
            );
            return std::ptr::null_mut();
        }

        let mut buffer = [0u8; 100];
        loop {
            let ret = fgets(
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len() as c_int,
                fp,
            );
            if ret.is_null() {
                break;
            }
            printf(b"%s\0".as_ptr() as *const c_char, buffer.as_ptr());
        }

        if ferror(fp) != 0 {
            // goto cleanup (fp is non-null, so fclose)
            fprintf(
                stderr,
                b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
                filename,
            );
            fclose(fp);
            return std::ptr::null_mut();
        }

        fp
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    unsafe {
        let res = forward_goto_example(num);
        if res == -1 {
            return -1;
        } else {
            printf(b"Goto output: %d\n\0".as_ptr() as *const c_char, res);
        }

        let out = open_with_cleanup(filename);
        if out.is_null() {
            return -2;
        } else {
            fclose(out);
        }

        0
    }
}
