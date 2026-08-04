// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior matches the original C program byte-for-byte.

use std::env;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::ptr;

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
    fn __errno_location() -> *mut c_int;
}

#[cfg(target_os = "macos")]
extern "C" {
    fn __error() -> *mut c_int;
}

fn errno_get() -> c_int {
    #[cfg(target_os = "macos")]
    unsafe {
        *__error()
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        *__errno_location()
    }
}

fn errno_set(val: c_int) {
    #[cfg(target_os = "macos")]
    unsafe {
        *__error() = val;
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        *__errno_location() = val;
    }
}

fn main() {
    // Collect raw argv as bytes (preserves whatever the OS passed).
    let args: Vec<Vec<u8>> = env::args_os()
        .map(|a| {
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStringExt;
                a.into_vec()
            }
            #[cfg(not(unix))]
            {
                a.to_string_lossy().into_owned().into_bytes()
            }
        })
        .collect();

    let argc = args.len() as c_int;

    // Convert each arg into a CString
    let cstrings: Vec<CString> = args
        .iter()
        .map(|a| CString::new(a.clone()).unwrap_or_else(|_| CString::new("").unwrap()))
        .collect();

    let argv0 = cstrings[0].as_ptr();

    let stderr = unsafe { stderr_handle() };

    if argc != 3 {
        let fmt = CString::new("Usage: %s base exponent\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), argv0);
        }
        std::process::exit(1);
    }

    // Convert base
    errno_set(0);
    let mut endptr1: *mut c_char = ptr::null_mut();
    let base = unsafe { strtod(cstrings[1].as_ptr(), &mut endptr1) };
    let errno_base = errno_get();
    if errno_base == libc::ERANGE {
        let fmt = CString::new("Range error while converting base '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), cstrings[1].as_ptr());
        }
        std::process::exit(1);
    } else if unsafe { *endptr1 } != 0 {
        let fmt = CString::new("Invalid numeric input for base: '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), cstrings[1].as_ptr());
        }
        std::process::exit(1);
    }

    // Convert exponent
    errno_set(0);
    let mut endptr2: *mut c_char = ptr::null_mut();
    let exponent = unsafe { strtod(cstrings[2].as_ptr(), &mut endptr2) };
    let errno_exp = errno_get();
    if errno_exp == libc::ERANGE {
        let fmt = CString::new("Range error while converting exponent '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), cstrings[2].as_ptr());
        }
        std::process::exit(1);
    } else if unsafe { *endptr2 } != 0 {
        let fmt = CString::new("Invalid numeric input for exponent: '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), cstrings[2].as_ptr());
        }
        std::process::exit(1);
    }

    // Calculate power
    errno_set(0);
    let result = unsafe { pow(base, exponent) };
    let errno_pow = errno_get();
    if errno_pow == libc::EDOM {
        let fmt =
            CString::new("Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n")
                .unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), base, exponent);
        }
        std::process::exit(1);
    } else if errno_pow == libc::ERANGE {
        let fmt =
            CString::new("Range error: pow(%.2f, %.2f) caused overflow or underflow.\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), base, exponent);
        }
        std::process::exit(1);
    }

    let fmt = CString::new("Result: %.2f\n").unwrap();
    unsafe {
        printf(fmt.as_ptr(), result);
    }
    std::process::exit(0);
}

unsafe fn stderr_handle() -> *mut libc::FILE {
    // libc exposes stderr as a function or static depending on platform.
    // Use the libc crate which provides a portable accessor.
    extern "C" {
        // On glibc / musl, `stderr` is a global FILE* symbol.
        static mut stderr: *mut libc::FILE;
    }
    stderr
}
