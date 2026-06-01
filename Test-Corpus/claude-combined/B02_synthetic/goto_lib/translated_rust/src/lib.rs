use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

use libc::{fclose, ferror, fgets, fopen, fprintf, printf, FILE};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn forward_goto_example(x: c_int) -> c_int {
    unsafe {
        if x < 0 {
            // forward goto: error
            fprintf(
                libc_stderr(),
                b"Error: negative input\n\0".as_ptr() as *const c_char,
            );
            return -1;
        }

        printf(b"Processing: %d\n\0".as_ptr() as *const c_char, x);
        x * 2
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    unsafe {
        let fp: *mut FILE = fopen(filename, b"r\0".as_ptr() as *const c_char);
        if fp.is_null() {
            // goto cleanup
            return cleanup(fp, filename);
        }

        let mut buffer: [c_char; 100] = [0; 100];
        while !fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp).is_null() {
            printf(b"%s\0".as_ptr() as *const c_char, buffer.as_ptr());
        }

        if ferror(fp) != 0 {
            return cleanup(fp, filename);
        }

        fp
    }
}

unsafe fn cleanup(fp: *mut FILE, filename: *const c_char) -> *mut FILE {
    unsafe {
        fprintf(
            libc_stderr(),
            b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
            filename,
        );
        if !fp.is_null() {
            fclose(fp);
        }
        ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
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

// Helper to access stderr for libc::fprintf in a portable way.
// On glibc, stderr is a pointer global. libc crate exposes it via `stderr()` on some
// platforms, but we go through the standard symbol lookup via the libc::stderr static.
#[cfg(target_os = "linux")]
fn libc_stderr() -> *mut FILE {
    extern "C" {
        static mut stderr: *mut FILE;
    }
    unsafe { stderr }
}

#[cfg(not(target_os = "linux"))]
fn libc_stderr() -> *mut FILE {
    extern "C" {
        static mut __stderrp: *mut FILE;
    }
    unsafe { __stderrp }
}
