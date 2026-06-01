// Copyright 2025 MIT Lincoln Laboratory
// Rust translation that reproduces the C `errno-pow` driver byte-for-byte.

use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::process::ExitCode;

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn pow(base: c_double, exp: c_double) -> c_double;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, fmt: *const c_char, ...) -> c_int;
    fn fflush(stream: *mut libc::FILE) -> c_int;
    fn __errno_location() -> *mut c_int;
}

#[allow(non_upper_case_globals)]
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

#[allow(non_upper_case_globals)]
fn errno_set(v: c_int) {
    unsafe { *__errno_location() = v };
}

fn stderr_ptr() -> *mut libc::FILE {
    extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

fn stdout_ptr() -> *mut libc::FILE {
    extern "C" {
        static mut stdout: *mut libc::FILE;
    }
    unsafe { stdout }
}

fn main() -> ExitCode {
    // Collect argv as raw C strings so strtod sees identical bytes.
    let raw_args: Vec<CString> = std::env::args_os()
        .map(|a| {
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStringExt;
                CString::new(a.into_vec()).expect("argv contains NUL byte")
            }
            #[cfg(not(unix))]
            {
                CString::new(a.to_string_lossy().into_owned()).expect("argv contains NUL byte")
            }
        })
        .collect();

    let argc = raw_args.len() as c_int;
    let argv0 = raw_args[0].as_ptr();

    let stderr = stderr_ptr();
    let stdout = stdout_ptr();

    if argc != 3 {
        let fmt = CString::new("Usage: %s base exponent\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), argv0);
            fflush(stderr);
        }
        return ExitCode::from(1);
    }

    let arg1 = raw_args[1].as_ptr();
    let arg2 = raw_args[2].as_ptr();

    // Convert base
    errno_set(0);
    let mut endptr1: *mut c_char = std::ptr::null_mut();
    let base = unsafe { strtod(arg1, &mut endptr1) };
    if errno_get() == libc::ERANGE {
        let fmt = CString::new("Range error while converting base '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), arg1);
            fflush(stderr);
        }
        return ExitCode::from(1);
    } else if unsafe { *endptr1 } != 0 {
        let fmt = CString::new("Invalid numeric input for base: '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), arg1);
            fflush(stderr);
        }
        return ExitCode::from(1);
    }

    // Convert exponent
    errno_set(0);
    let mut endptr2: *mut c_char = std::ptr::null_mut();
    let exponent = unsafe { strtod(arg2, &mut endptr2) };
    if errno_get() == libc::ERANGE {
        let fmt = CString::new("Range error while converting exponent '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), arg2);
            fflush(stderr);
        }
        return ExitCode::from(1);
    } else if unsafe { *endptr2 } != 0 {
        let fmt = CString::new("Invalid numeric input for exponent: '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), arg2);
            fflush(stderr);
        }
        return ExitCode::from(1);
    }

    // Calculate power
    errno_set(0);
    let result = unsafe { pow(base, exponent) };
    let err = errno_get();
    if err == libc::EDOM {
        let fmt = CString::new(
            "Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n",
        )
        .unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), base, exponent);
            fflush(stderr);
        }
        return ExitCode::from(1);
    } else if err == libc::ERANGE {
        let fmt =
            CString::new("Range error: pow(%.2f, %.2f) caused overflow or underflow.\n").unwrap();
        unsafe {
            fprintf(stderr, fmt.as_ptr(), base, exponent);
            fflush(stderr);
        }
        return ExitCode::from(1);
    }

    let fmt = CString::new("Result: %.2f\n").unwrap();
    unsafe {
        printf(fmt.as_ptr(), result);
        fflush(stdout);
    }
    ExitCode::from(0)
}
