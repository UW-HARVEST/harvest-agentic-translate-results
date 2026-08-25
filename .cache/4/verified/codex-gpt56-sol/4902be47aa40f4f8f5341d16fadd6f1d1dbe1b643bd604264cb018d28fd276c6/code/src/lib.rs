use std::ffi::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct CFile {
    _private: [u8; 0],
}

unsafe extern "C" {
    static mut stderr: *mut CFile;

    fn fclose(stream: *mut CFile) -> c_int;
    fn ferror(stream: *mut CFile) -> c_int;
    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut CFile) -> *mut c_char;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut CFile;
    fn fprintf(stream: *mut CFile, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        unsafe {
            fprintf(stderr, c"Error: negative input\n".as_ptr());
        }
        return -1;
    }

    unsafe {
        printf(c"Processing: %d\n".as_ptr(), x);
    }
    x.wrapping_mul(2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut CFile {
    let fp = unsafe { fopen(filename, c"r".as_ptr()) };
    if fp.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Error: opening or processing file %s\n".as_ptr(),
                filename,
            );
        }
        return ptr::null_mut();
    }

    let mut buffer = [0 as c_char; 100];
    while !unsafe { fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp) }.is_null() {
        unsafe {
            printf(c"%s".as_ptr(), buffer.as_ptr());
        }
    }

    if unsafe { ferror(fp) } != 0 {
        unsafe {
            fprintf(
                stderr,
                c"Error: opening or processing file %s\n".as_ptr(),
                filename,
            );
            fclose(fp);
        }
        return ptr::null_mut();
    }

    fp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let result = unsafe { forward_goto_example(num) };
    if result == -1 {
        return -1;
    }

    unsafe {
        printf(c"Goto output: %d\n".as_ptr(), result);
    }

    let output = unsafe { open_with_cleanup(filename) };
    if output.is_null() {
        return -2;
    }

    unsafe {
        fclose(output);
    }
    0
}
