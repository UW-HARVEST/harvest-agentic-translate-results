// Translated from c_src/src/main.c
// Library translation: byte-identical output to the original C program.

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut libc::FILE) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn memset(
        s: *mut std::ffi::c_void,
        c: c_int,
        n: libc::size_t,
    ) -> *mut std::ffi::c_void;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: libc::size_t) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            // Use "%s\n" format to match the C code byte-for-byte.
            let fmt = b"%s\n\0".as_ptr() as *const c_char;
            printf(fmt, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut data: c_int = -1;

    {
        let mut input_buffer: [c_char; 14] = [0; 14];
        unsafe {
            let stdin_stream: *mut libc::FILE = libc_stdin();
            let res = fgets(input_buffer.as_mut_ptr(), 14, stdin_stream);
            if !res.is_null() {
                /* Convert to int */
                data = atoi(input_buffer.as_ptr());
            } else {
                let msg = b"fgets() failed.\0".as_ptr() as *const c_char;
                printLine(msg);
            }
        }
    }

    {
        let mut source: [c_char; 100] = [0; 100];
        let mut dest: [c_char; 100] = [0; 100];
        unsafe {
            memset(
                source.as_mut_ptr() as *mut std::ffi::c_void,
                b'A' as c_int,
                (100 - 1) as libc::size_t,
            );
            source[100 - 1] = 0;
            if data < 100 {
                // Mirror C behavior exactly. Note: if data is negative this
                // matches the original C's (undefined) behavior.
                strncpy(
                    dest.as_mut_ptr(),
                    source.as_ptr(),
                    data as libc::size_t,
                );
                let p = dest.as_mut_ptr().offset(data as isize);
                *p = 0;
            }
            printLine(dest.as_ptr());
        }
    }

    0
}

/// Get the platform's stdin FILE*. Wraps libc differences across platforms.
#[inline]
fn libc_stdin() -> *mut libc::FILE {
    // libc provides a function to access the stdin handle on most platforms.
    unsafe { libc_stdin_impl() }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn libc_stdin_impl() -> *mut libc::FILE {
    extern "C" {
        static mut stdin: *mut libc::FILE;
    }
    stdin
}

#[cfg(target_os = "macos")]
unsafe fn libc_stdin_impl() -> *mut libc::FILE {
    extern "C" {
        static mut __stdinp: *mut libc::FILE;
    }
    __stdinp
}

#[cfg(target_os = "windows")]
unsafe fn libc_stdin_impl() -> *mut libc::FILE {
    extern "C" {
        fn __acrt_iob_func(idx: u32) -> *mut libc::FILE;
    }
    __acrt_iob_func(0)
}
