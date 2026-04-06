use std::ffi::c_int;
use std::os::raw::c_char;

use libc::{fclose, ferror, fgets, fopen, fprintf, printf, FILE};

unsafe extern "C" {
    static stderr: *mut FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        unsafe {
            fprintf(stderr, b"Error: negative input\n\0".as_ptr() as *const c_char);
        }
        return -1;
    }

    unsafe {
        printf(b"Processing: %d\n\0".as_ptr() as *const c_char, x);
    }
    x * 2
}

#[unsafe(no_mangle)]
pub extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    let fp = unsafe { fopen(filename, b"r\0".as_ptr() as *const c_char) };
    if fp.is_null() {
        unsafe {
            fprintf(
                stderr,
                b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
                filename,
            );
        }
        return std::ptr::null_mut();
    }

    let mut buffer = [0u8; 100];
    loop {
        let ret = unsafe { fgets(buffer.as_mut_ptr() as *mut c_char, 100, fp) };
        if ret.is_null() {
            break;
        }
        unsafe {
            printf(b"%s\0".as_ptr() as *const c_char, buffer.as_ptr());
        }
    }

    if unsafe { ferror(fp) } != 0 {
        unsafe {
            fprintf(
                stderr,
                b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
                filename,
            );
            fclose(fp);
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
            printf(b"Goto output: %d\n\0".as_ptr() as *const c_char, res);
        }
    }

    let out = open_with_cleanup(filename);
    if out.is_null() {
        return -2;
    } else {
        unsafe {
            fclose(out);
        }
    }

    0
}
