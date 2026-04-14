use libc::{FILE, fclose, ferror, fgets, fopen, fprintf, printf, stderr};
use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_int;
use std::ptr;

fn forward_goto_example_impl(x: c_int) -> c_int {
    if x < 0 {
        let msg = c"Error: negative input\n";
        unsafe {
            fprintf(stderr, msg.as_ptr());
        }
        return -1;
    }

    let fmt = c"Processing: %d\n";
    unsafe {
        printf(fmt.as_ptr(), x);
    }
    x * 2
}

fn open_with_cleanup_impl(filename: *const c_char) -> *mut FILE {
    let mode = c"r";
    let mut fp = unsafe { fopen(filename, mode.as_ptr()) };
    if fp.is_null() {
        let fmt = c"Error: opening or processing file %s\n";
        unsafe {
            fprintf(stderr, fmt.as_ptr(), filename);
        }
        return ptr::null_mut();
    }

    let mut buffer = [0 as c_char; 100];
    loop {
        let line = unsafe { fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp) };
        if line.is_null() {
            break;
        }
        let fmt = c"%s";
        unsafe {
            printf(fmt.as_ptr(), buffer.as_ptr());
        }
    }

    if unsafe { ferror(fp) } != 0 {
        let fmt = c"Error: opening or processing file %s\n";
        unsafe {
            fprintf(stderr, fmt.as_ptr(), filename);
            fclose(fp);
        }
        fp = ptr::null_mut();
    }

    fp
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    if filename.is_null() {
        return -2;
    }

    let res = forward_goto_example_impl(num);
    if res == -1 {
        return -1;
    } else {
        let fmt = c"Goto output: %d\n";
        unsafe {
            printf(fmt.as_ptr(), res);
        }
    }

    let out = open_with_cleanup_impl(filename);
    if out.is_null() {
        -2
    } else {
        unsafe {
            fclose(out);
        }
        0
    }
}
