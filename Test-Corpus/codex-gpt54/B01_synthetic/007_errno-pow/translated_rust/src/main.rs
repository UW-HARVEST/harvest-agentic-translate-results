use std::env;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::process;
use std::ptr;

unsafe extern "C" {
    fn fprintf(stream: *mut libc::FILE, format: *const libc::c_char, ...) -> libc::c_int;
    fn printf(format: *const libc::c_char, ...) -> libc::c_int;
    fn strtod(
        nptr: *const libc::c_char,
        endptr: *mut *mut libc::c_char,
    ) -> libc::c_double;
    fn pow(x: libc::c_double, y: libc::c_double) -> libc::c_double;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn __errno_location() -> *mut libc::c_int;

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn __error() -> *mut libc::c_int;

    static mut stderr: *mut libc::FILE;
}

fn c_string_from_os_arg(arg: &std::ffi::OsStr) -> CString {
    CString::new(arg.as_bytes()).expect("argv contains interior NUL")
}

fn errno_ptr() -> *mut libc::c_int {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        __errno_location()
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    unsafe {
        __error()
    }
}

fn set_errno(value: libc::c_int) {
    unsafe {
        *errno_ptr() = value;
    }
}

fn get_errno() -> libc::c_int {
    unsafe { *errno_ptr() }
}

fn main() {
    let args: Vec<_> = env::args_os().collect();

    if args.len() != 3 {
        let argv0 = c_string_from_os_arg(args.first().map_or(std::ffi::OsStr::new(""), |s| s));
        let usage = CString::new("Usage: %s base exponent\n").unwrap();
        unsafe {
            fprintf(stderr, usage.as_ptr(), argv0.as_ptr());
        }
        process::exit(1);
    }

    let base_arg = c_string_from_os_arg(&args[1]);
    let exponent_arg = c_string_from_os_arg(&args[2]);

    let mut endptr1: *mut libc::c_char = ptr::null_mut();
    set_errno(0);
    let base = unsafe { strtod(base_arg.as_ptr(), &mut endptr1) };
    if get_errno() == libc::ERANGE {
        let msg = CString::new("Range error while converting base '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, msg.as_ptr(), base_arg.as_ptr());
        }
        process::exit(1);
    } else if unsafe { *endptr1 } != 0 {
        let msg = CString::new("Invalid numeric input for base: '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, msg.as_ptr(), base_arg.as_ptr());
        }
        process::exit(1);
    }

    let mut endptr2: *mut libc::c_char = ptr::null_mut();
    set_errno(0);
    let exponent = unsafe { strtod(exponent_arg.as_ptr(), &mut endptr2) };
    if get_errno() == libc::ERANGE {
        let msg = CString::new("Range error while converting exponent '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, msg.as_ptr(), exponent_arg.as_ptr());
        }
        process::exit(1);
    } else if unsafe { *endptr2 } != 0 {
        let msg = CString::new("Invalid numeric input for exponent: '%s'\n").unwrap();
        unsafe {
            fprintf(stderr, msg.as_ptr(), exponent_arg.as_ptr());
        }
        process::exit(1);
    }

    set_errno(0);
    let result = unsafe { pow(base, exponent) };
    if get_errno() == libc::EDOM {
        let msg = CString::new(
            "Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n",
        )
        .unwrap();
        unsafe {
            fprintf(stderr, msg.as_ptr(), base, exponent);
        }
        process::exit(1);
    } else if get_errno() == libc::ERANGE {
        let msg =
            CString::new("Range error: pow(%.2f, %.2f) caused overflow or underflow.\n").unwrap();
        unsafe {
            fprintf(stderr, msg.as_ptr(), base, exponent);
        }
        process::exit(1);
    }

    let msg = CString::new("Result: %.2f\n").unwrap();
    unsafe {
        printf(msg.as_ptr(), result);
    }
}
