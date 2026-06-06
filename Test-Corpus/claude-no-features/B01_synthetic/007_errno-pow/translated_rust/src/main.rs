// Translated from c_src/src/main.c
// Takes two arguments, a base and an exponent, and prints base^exponent

use std::env;
use std::ffi::CString;
use std::process::ExitCode;

extern "C" {
    fn strtod(nptr: *const libc::c_char, endptr: *mut *mut libc::c_char) -> libc::c_double;
    fn pow(x: libc::c_double, y: libc::c_double) -> libc::c_double;
    fn __errno_location() -> *mut libc::c_int;
}

const ERANGE: libc::c_int = 34;
const EDOM: libc::c_int = 33;

fn get_errno() -> libc::c_int {
    unsafe { *__errno_location() }
}

fn set_errno(val: libc::c_int) {
    unsafe { *__errno_location() = val };
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    // Get program name (argv[0])
    let prog_name = if argc > 0 { args[0].clone() } else { String::new() };

    if argc != 3 {
        eprintln!("Usage: {} base exponent", prog_name);
        return ExitCode::from(1);
    }

    // Convert base
    let arg1 = match CString::new(args[1].clone()) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Invalid numeric input for base: '{}'", args[1]);
            return ExitCode::from(1);
        }
    };
    set_errno(0);
    let mut endptr1: *mut libc::c_char = std::ptr::null_mut();
    let base = unsafe { strtod(arg1.as_ptr(), &mut endptr1 as *mut *mut libc::c_char) };
    if get_errno() == ERANGE {
        eprintln!("Range error while converting base '{}'", args[1]);
        return ExitCode::from(1);
    } else if unsafe { *endptr1 } != 0 {
        eprintln!("Invalid numeric input for base: '{}'", args[1]);
        return ExitCode::from(1);
    }

    // Convert exponent
    let arg2 = match CString::new(args[2].clone()) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Invalid numeric input for exponent: '{}'", args[2]);
            return ExitCode::from(1);
        }
    };
    set_errno(0);
    let mut endptr2: *mut libc::c_char = std::ptr::null_mut();
    let exponent = unsafe { strtod(arg2.as_ptr(), &mut endptr2 as *mut *mut libc::c_char) };
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
            "Domain error: pow({}, {}) is undefined in the real number domain.",
            format_f64_2(base),
            format_f64_2(exponent)
        );
        return ExitCode::from(1);
    } else if err == ERANGE {
        eprintln!(
            "Range error: pow({}, {}) caused overflow or underflow.",
            format_f64_2(base),
            format_f64_2(exponent)
        );
        return ExitCode::from(1);
    }

    println!("Result: {}", format_f64_2(result));
    ExitCode::from(0)
}

// Format a f64 like C's "%.2f" — uses libc snprintf to ensure byte-identical output
fn format_f64_2(v: f64) -> String {
    // Buffer must be large enough for any %.2f double (largest ~ 1.8e308 → ~310 chars + sign + dot + 2 + null)
    let mut buf = vec![0u8; 512];
    let fmt = b"%.2f\0";
    let n = unsafe {
        libc::snprintf(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            fmt.as_ptr() as *const libc::c_char,
            v,
        )
    };
    if n < 0 {
        return String::new();
    }
    let len = (n as usize).min(buf.len() - 1);
    String::from_utf8_lossy(&buf[..len]).into_owned()
}
