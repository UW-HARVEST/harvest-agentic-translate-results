use std::ffi::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

unsafe extern "C" {
    static mut stderr: *mut FILE;

    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut FILE) -> *mut c_char;
    fn ferror(stream: *mut FILE) -> c_int;
    fn fclose(stream: *mut FILE) -> c_int;
}

const MODE_READ: &[u8] = b"r\0";
const PROCESSING_FMT: &[u8] = b"Processing: %d\n\0";
const NEGATIVE_INPUT_FMT: &[u8] = b"Error: negative input\n\0";
const STRING_FMT: &[u8] = b"%s\0";
const OPEN_OR_PROCESSING_ERROR_FMT: &[u8] = b"Error: opening or processing file %s\n\0";
const GOTO_OUTPUT_FMT: &[u8] = b"Goto output: %d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        // Preserve the original control flow and output stream behavior.
        unsafe {
            fprintf(
                stderr,
                NEGATIVE_INPUT_FMT.as_ptr().cast::<c_char>(),
            );
        }
        return -1;
    }

    unsafe {
        printf(PROCESSING_FMT.as_ptr().cast::<c_char>(), x);
    }
    x * 2
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    let fp = unsafe { fopen(filename, MODE_READ.as_ptr().cast::<c_char>()) };
    if fp.is_null() {
        unsafe {
            fprintf(
                stderr,
                OPEN_OR_PROCESSING_ERROR_FMT.as_ptr().cast::<c_char>(),
                filename,
            );
        }
        return ptr::null_mut();
    }

    let mut buffer = [0 as c_char; 100];
    while unsafe { !fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp).is_null() } {
        unsafe {
            printf(STRING_FMT.as_ptr().cast::<c_char>(), buffer.as_ptr());
        }
    }

    if unsafe { ferror(fp) } != 0 {
        unsafe {
            fprintf(
                stderr,
                OPEN_OR_PROCESSING_ERROR_FMT.as_ptr().cast::<c_char>(),
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
    let res = unsafe { forward_goto_example(num) };
    if res == -1 {
        return -1;
    } else {
        unsafe {
            printf(GOTO_OUTPUT_FMT.as_ptr().cast::<c_char>(), res);
        }
    }

    let out = unsafe { open_with_cleanup(filename) };
    if out.is_null() {
        -2
    } else {
        unsafe {
            fclose(out);
        }
        0
    }
}
