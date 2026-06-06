use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;

extern "C" {
    static stderr: *mut c_void;

    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut c_void) -> *mut c_char;
    fn ferror(stream: *mut c_void) -> c_int;
}

fn forward_goto_example(x: c_int) -> c_int {
    unsafe {
        if x < 0 {
            // goto error;
            fprintf(
                stderr,
                b"Error: negative input\n\0".as_ptr() as *const c_char,
            );
            return -1;
        }

        printf(b"Processing: %d\n\0".as_ptr() as *const c_char, x);
        x.wrapping_mul(2)
    }
}

fn open_with_cleanup(filename: *const c_char) -> *mut c_void {
    unsafe {
        let fp = fopen(filename, b"r\0".as_ptr() as *const c_char);
        if fp.is_null() {
            // goto cleanup;
            fprintf(
                stderr,
                b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
                filename,
            );
            if !fp.is_null() {
                fclose(fp);
            }
            return std::ptr::null_mut();
        }

        let mut buffer: [c_char; 100] = [0; 100];
        while !fgets(buffer.as_mut_ptr(), 100, fp).is_null() {
            printf(b"%s\0".as_ptr() as *const c_char, buffer.as_ptr());
        }

        if ferror(fp) != 0 {
            // goto cleanup;
            fprintf(
                stderr,
                b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
                filename,
            );
            if !fp.is_null() {
                fclose(fp);
            }
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
