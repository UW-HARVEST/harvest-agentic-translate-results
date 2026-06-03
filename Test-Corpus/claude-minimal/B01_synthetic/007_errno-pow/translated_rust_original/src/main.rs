// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::env;
use std::ffi::CString;
use std::process::ExitCode;
use std::ptr;

use libc::{c_char, c_int, EDOM, ERANGE};

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    #[cfg(target_os = "linux")]
    fn __errno_location() -> *mut c_int;
    #[cfg(target_os = "macos")]
    fn __error() -> *mut c_int;
}

#[cfg(target_os = "linux")]
fn errno_ptr() -> *mut c_int {
    unsafe { __errno_location() }
}

#[cfg(target_os = "macos")]
fn errno_ptr() -> *mut c_int {
    unsafe { __error() }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn errno_ptr() -> *mut c_int {
    // Fallback - use libc's __errno_location if available; otherwise this won't compile.
    // Most Unix targets are covered above.
    unsafe extern "C" {
        fn __errno_location() -> *mut c_int;
    }
    unsafe { __errno_location() }
}

fn set_errno(value: c_int) {
    unsafe {
        *errno_ptr() = value;
    }
}

fn get_errno() -> c_int {
    unsafe { *errno_ptr() }
}

// Takes two arguments, a base and an exponent, and prints base^exponent
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 3 {
        let prog = args.get(0).map(|s| s.as_str()).unwrap_or("driver");
        eprintln!("Usage: {} base exponent", prog);
        return ExitCode::from(1);
    }

    // Convert base
    let arg1 = match CString::new(args[1].as_str()) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Invalid numeric input for base: '{}'", args[1]);
            return ExitCode::from(1);
        }
    };
    let mut endptr1: *mut c_char = ptr::null_mut();
    set_errno(0);
    let base = unsafe { strtod(arg1.as_ptr(), &mut endptr1 as *mut *mut c_char) };
    if get_errno() == ERANGE {
        eprintln!("Range error while converting base '{}'", args[1]);
        return ExitCode::from(1);
    } else if unsafe { *endptr1 } != 0 {
        eprintln!("Invalid numeric input for base: '{}'", args[1]);
        return ExitCode::from(1);
    }

    // Convert exponent
    let arg2 = match CString::new(args[2].as_str()) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Invalid numeric input for exponent: '{}'", args[2]);
            return ExitCode::from(1);
        }
    };
    let mut endptr2: *mut c_char = ptr::null_mut();
    set_errno(0);
    let exponent = unsafe { strtod(arg2.as_ptr(), &mut endptr2 as *mut *mut c_char) };
    if get_errno() == ERANGE {
        eprintln!("Range error while converting exponent '{}'", args[2]);
        return ExitCode::from(1);
    } else if unsafe { *endptr2 } != 0 {
        eprintln!("Invalid numeric input for exponent: '{}'", args[2]);
        return ExitCode::from(1);
    }

    // Calculate power
    set_errno(0);
    let result = unsafe { pow(base, exponent) };
    let err = get_errno();
    if err == EDOM {
        eprintln!(
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        return ExitCode::from(1);
    } else if err == ERANGE {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        return ExitCode::from(1);
    }

    println!("Result: {:.2}", result);
    ExitCode::from(0)
}
