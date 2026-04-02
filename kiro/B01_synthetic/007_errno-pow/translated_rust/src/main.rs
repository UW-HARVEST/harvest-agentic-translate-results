use std::env;
use std::ffi::CString;
use std::process;

extern "C" {
    fn strtod(nptr: *const libc::c_char, endptr: *mut *mut libc::c_char) -> f64;
    fn pow(base: f64, exp: f64) -> f64;
}

fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

fn set_errno(val: i32) {
    unsafe {
        *libc::__errno_location() = val;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} base exponent", args[0]);
        process::exit(1);
    }

    // Convert base
    let base = {
        let cstr = CString::new(args[1].as_str()).unwrap();
        let mut endptr: *mut libc::c_char = std::ptr::null_mut();
        set_errno(0);
        let val = unsafe { strtod(cstr.as_ptr(), &mut endptr) };
        if errno() == libc::ERANGE {
            eprintln!("Range error while converting base '{}'", args[1]);
            process::exit(1);
        } else if unsafe { *endptr } != 0 {
            eprintln!("Invalid numeric input for base: '{}'", args[1]);
            process::exit(1);
        }
        val
    };

    // Convert exponent
    let exponent = {
        let cstr = CString::new(args[2].as_str()).unwrap();
        let mut endptr: *mut libc::c_char = std::ptr::null_mut();
        set_errno(0);
        let val = unsafe { strtod(cstr.as_ptr(), &mut endptr) };
        if errno() == libc::ERANGE {
            eprintln!("Range error while converting exponent '{}'", args[2]);
            process::exit(1);
        } else if unsafe { *endptr } != 0 {
            eprintln!("Invalid numeric input for exponent: '{}'", args[2]);
            process::exit(1);
        }
        val
    };

    // Calculate power
    set_errno(0);
    let result = unsafe { pow(base, exponent) };
    if errno() == libc::EDOM {
        eprintln!(
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        process::exit(1);
    } else if errno() == libc::ERANGE {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        process::exit(1);
    }

    println!("Result: {:.2}", result);
}
