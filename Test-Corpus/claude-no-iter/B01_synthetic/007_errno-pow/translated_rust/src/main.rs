// Copyright 2025 MIT Lincoln Laboratory
// (License header preserved from original C source)
//
// Takes two arguments, a base and an exponent, and prints base^exponent.
// This is a Rust port of the original C program. We use libc functions
// (strtod, pow, printf, fprintf) so that parsing, math, and formatting
// behavior is byte-identical to the C implementation.

use std::env;
use std::ffi::CString;
use std::process::ExitCode;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
}

// Errno helpers - access the platform's errno location.
fn errno_get() -> c_int {
    // SAFETY: __errno_location() (Linux) / __error() (macOS) returns a
    // valid pointer to a thread-local int.
    unsafe { *errno_location() }
}

fn errno_set(value: c_int) {
    // SAFETY: see above.
    unsafe { *errno_location() = value };
}

#[cfg(any(target_os = "linux", target_os = "android"))]
extern "C" {
    #[link_name = "__errno_location"]
    fn errno_location() -> *mut c_int;
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
extern "C" {
    #[link_name = "__error"]
    fn errno_location() -> *mut c_int;
}

const ERANGE: c_int = libc::ERANGE;
const EDOM: c_int = libc::EDOM;

fn main() -> ExitCode {
    // Collect command line arguments. We need raw NUL-terminated bytes to
    // pass to libc::strtod, so we keep CString copies of each arg.
    let args_os: Vec<_> = env::args_os().collect();
    let argc = args_os.len();

    // Build CStrings for each argument (handles arbitrary OS bytes that are
    // NUL-free).
    let argv_cstrs: Vec<CString> = args_os
        .iter()
        .map(|s| {
            let bytes = {
                #[cfg(unix)]
                {
                    use std::os::unix::ffi::OsStrExt;
                    s.as_os_str().as_bytes().to_vec()
                }
                #[cfg(not(unix))]
                {
                    s.to_string_lossy().into_owned().into_bytes()
                }
            };
            CString::new(bytes).expect("argument contained interior NUL byte")
        })
        .collect();

    // Pointer to argv[0] for usage message.
    let prog_name_ptr = argv_cstrs[0].as_ptr();

    if argc != 3 {
        let fmt = CString::new("Usage: %s base exponent\n").unwrap();
        // SAFETY: stderr is a valid FILE*; format string is a valid C string.
        unsafe {
            fprintf(
                libc_stderr(),
                fmt.as_ptr(),
                prog_name_ptr,
            );
        }
        return ExitCode::from(1);
    }

    // Convert base
    errno_set(0);
    let mut endptr1: *mut c_char = std::ptr::null_mut();
    // SAFETY: argv_cstrs[1] is a valid NUL-terminated C string; endptr1 is
    // a valid out-pointer.
    let base = unsafe { strtod(argv_cstrs[1].as_ptr(), &mut endptr1 as *mut *mut c_char) };
    if errno_get() == ERANGE {
        let fmt = CString::new("Range error while converting base '%s'\n").unwrap();
        // SAFETY: see above.
        unsafe {
            fprintf(libc_stderr(), fmt.as_ptr(), argv_cstrs[1].as_ptr());
        }
        return ExitCode::from(1);
    } else if unsafe { *endptr1 } != 0 {
        let fmt = CString::new("Invalid numeric input for base: '%s'\n").unwrap();
        // SAFETY: see above.
        unsafe {
            fprintf(libc_stderr(), fmt.as_ptr(), argv_cstrs[1].as_ptr());
        }
        return ExitCode::from(1);
    }

    // Convert exponent
    errno_set(0);
    let mut endptr2: *mut c_char = std::ptr::null_mut();
    // SAFETY: see above.
    let exponent = unsafe { strtod(argv_cstrs[2].as_ptr(), &mut endptr2 as *mut *mut c_char) };
    if errno_get() == ERANGE {
        let fmt = CString::new("Range error while converting exponent '%s'\n").unwrap();
        // SAFETY: see above.
        unsafe {
            fprintf(libc_stderr(), fmt.as_ptr(), argv_cstrs[2].as_ptr());
        }
        return ExitCode::from(1);
    } else if unsafe { *endptr2 } != 0 {
        let fmt = CString::new("Invalid numeric input for exponent: '%s'\n").unwrap();
        // SAFETY: see above.
        unsafe {
            fprintf(libc_stderr(), fmt.as_ptr(), argv_cstrs[2].as_ptr());
        }
        return ExitCode::from(1);
    }

    // Calculate power
    errno_set(0);
    // SAFETY: pow has no preconditions on f64 inputs.
    let result = unsafe { pow(base, exponent) };
    let err = errno_get();
    if err == EDOM {
        let fmt =
            CString::new("Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n")
                .unwrap();
        // SAFETY: see above.
        unsafe {
            fprintf(libc_stderr(), fmt.as_ptr(), base, exponent);
        }
        return ExitCode::from(1);
    } else if err == ERANGE {
        let fmt =
            CString::new("Range error: pow(%.2f, %.2f) caused overflow or underflow.\n").unwrap();
        // SAFETY: see above.
        unsafe {
            fprintf(libc_stderr(), fmt.as_ptr(), base, exponent);
        }
        return ExitCode::from(1);
    }

    let fmt = CString::new("Result: %.2f\n").unwrap();
    // SAFETY: see above.
    unsafe {
        printf(fmt.as_ptr(), result);
    }
    ExitCode::from(0)
}

// Returns the libc `FILE *stderr` for use with fprintf. On Linux glibc this
// is the symbol `stderr`. We rely on the libc crate to expose it portably.
fn libc_stderr() -> *mut libc::FILE {
    // SAFETY: libc::stderr() is safe to call; returns the standard stderr
    // FILE* for this process.
    extern "C" {
        // The libc crate exposes this via libc::stderr but it is a function
        // returning the FILE*. We use the helper from the libc crate.
    }
    // Use libc crate accessor.
    // On glibc/musl, `stderr` is a static variable; libc crate provides it.
    // We must read it via the unsafe accessor.
    unsafe { libc_stderr_inner() }
}

unsafe fn libc_stderr_inner() -> *mut libc::FILE {
    // libc::stderr is defined as a static for glibc/musl/macOS through the
    // libc crate. Some platforms expose it differently; try the standard
    // approach.
    extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    stderr
}
