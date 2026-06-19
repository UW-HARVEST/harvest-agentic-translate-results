use std::ffi::{c_char, c_int};

const READ_MODE: &[u8] = b"r\0";
const PROCESSING_FMT: &[u8] = b"Processing: %d\n\0";
const NEGATIVE_ERROR: &[u8] = b"Error: negative input\n\0";
const FILE_ERROR_FMT: &[u8] = b"Error: opening or processing file %s\n\0";
const STRING_FMT: &[u8] = b"%s\0";
const GOTO_OUTPUT_FMT: &[u8] = b"Goto output: %d\n\0";

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        unsafe {
            libc::fprintf(stderr, NEGATIVE_ERROR.as_ptr().cast::<c_char>());
        }
        return -1;
    }

    unsafe {
        libc::printf(PROCESSING_FMT.as_ptr().cast::<c_char>(), x);
    }
    x.wrapping_mul(2)
}

#[unsafe(no_mangle)]
pub extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut libc::FILE {
    let fp = unsafe { libc::fopen(filename, READ_MODE.as_ptr().cast::<c_char>()) };
    if fp.is_null() {
        unsafe {
            libc::fprintf(
                stderr,
                FILE_ERROR_FMT.as_ptr().cast::<c_char>(),
                filename,
            );
        }
        return std::ptr::null_mut();
    }

    let mut buffer = [0 as c_char; 100];
    while unsafe { !libc::fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp).is_null() } {
        unsafe {
            libc::printf(STRING_FMT.as_ptr().cast::<c_char>(), buffer.as_ptr());
        }
    }

    if unsafe { libc::ferror(fp) } != 0 {
        unsafe {
            libc::fprintf(
                stderr,
                FILE_ERROR_FMT.as_ptr().cast::<c_char>(),
                filename,
            );
            libc::fclose(fp);
        }
        return std::ptr::null_mut();
    }

    fp
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        unsafe {
            libc::printf(GOTO_OUTPUT_FMT.as_ptr().cast::<c_char>(), res);
        }
    }

    let out = open_with_cleanup(filename);
    if out.is_null() {
        return -2;
    } else {
        unsafe {
            libc::fclose(out);
        }
    }

    0
}
