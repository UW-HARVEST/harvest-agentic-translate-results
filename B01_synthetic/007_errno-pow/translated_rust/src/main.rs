use std::env;
use std::process;

extern "C" {
    fn strtod(nptr: *const libc::c_char, endptr: *mut *mut libc::c_char) -> f64;
    fn pow(base: f64, exp: f64) -> f64;
}

fn set_errno(val: i32) {
    unsafe { *libc::__errno_location() = val; }
}

fn get_errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprint!("Usage: {} base exponent\n", args[0]);
        process::exit(1);
    }

    // Convert base
    let base: f64;
    {
        let cstr = std::ffi::CString::new(args[1].as_str()).unwrap();
        let mut endptr: *mut libc::c_char = std::ptr::null_mut();
        set_errno(0);
        base = unsafe { strtod(cstr.as_ptr(), &mut endptr) };
        if get_errno() == libc::ERANGE {
            eprint!("Range error while converting base '{}'\n", args[1]);
            process::exit(1);
        } else if unsafe { *endptr } != 0 {
            eprint!("Invalid numeric input for base: '{}'\n", args[1]);
            process::exit(1);
        }
    }

    // Convert exponent
    let exponent: f64;
    {
        let cstr = std::ffi::CString::new(args[2].as_str()).unwrap();
        let mut endptr: *mut libc::c_char = std::ptr::null_mut();
        set_errno(0);
        exponent = unsafe { strtod(cstr.as_ptr(), &mut endptr) };
        if get_errno() == libc::ERANGE {
            eprint!("Range error while converting exponent '{}'\n", args[2]);
            process::exit(1);
        } else if unsafe { *endptr } != 0 {
            eprint!("Invalid numeric input for exponent: '{}'\n", args[2]);
            process::exit(1);
        }
    }

    // Calculate power
    set_errno(0);
    let result = unsafe { pow(base, exponent) };
    if get_errno() == libc::EDOM {
        eprint!("Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.\n", base, exponent);
        process::exit(1);
    } else if get_errno() == libc::ERANGE {
        eprint!("Range error: pow({:.2}, {:.2}) caused overflow or underflow.\n", base, exponent);
        process::exit(1);
    }

    print!("Result: {:.2}\n", result);
}
