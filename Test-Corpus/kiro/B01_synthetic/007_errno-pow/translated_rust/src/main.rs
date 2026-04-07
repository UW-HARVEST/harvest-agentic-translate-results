use std::env;
use std::ffi::CString;
use std::process;

extern "C" {
    fn pow(x: f64, y: f64) -> f64;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} base exponent", args[0]);
        process::exit(1);
    }

    // Convert base
    let base = parse_double(&args[1], "base");
    // Convert exponent
    let exponent = parse_double(&args[2], "exponent");

    // Calculate power
    unsafe {
        *libc::__errno_location() = 0;
    }
    let result = unsafe { pow(base, exponent) };
    let err = unsafe { *libc::__errno_location() };
    if err == libc::EDOM {
        eprintln!(
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        process::exit(1);
    } else if err == libc::ERANGE {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        process::exit(1);
    }

    println!("Result: {:.2}", result);
}

fn parse_double(s: &str, name: &str) -> f64 {
    let cstr = CString::new(s).unwrap();
    let mut endptr: *mut libc::c_char = std::ptr::null_mut();
    unsafe {
        *libc::__errno_location() = 0;
    }
    let val = unsafe { libc::strtod(cstr.as_ptr(), &mut endptr) };
    let err = unsafe { *libc::__errno_location() };
    if err == libc::ERANGE {
        eprintln!("Range error while converting {} '{}'", name, s);
        process::exit(1);
    } else if unsafe { *endptr } != 0 {
        eprintln!("Invalid numeric input for {}: '{}'", name, s);
        process::exit(1);
    }
    val
}
