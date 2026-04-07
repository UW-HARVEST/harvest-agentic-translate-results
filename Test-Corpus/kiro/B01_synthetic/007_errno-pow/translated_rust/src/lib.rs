use std::ffi::{CStr, CString};

extern "C" {
    fn pow(x: f64, y: f64) -> f64;
}

fn parse_double(s: &str, name: &str) -> Result<f64, String> {
    let cstr = CString::new(s).unwrap();
    let mut endptr: *mut libc::c_char = std::ptr::null_mut();
    unsafe { *libc::__errno_location() = 0; }
    let val = unsafe { libc::strtod(cstr.as_ptr(), &mut endptr) };
    let err = unsafe { *libc::__errno_location() };
    if err == libc::ERANGE {
        Err(format!("Range error while converting {} '{}'", name, s))
    } else if unsafe { *endptr } != 0 {
        Err(format!("Invalid numeric input for {}: '{}'", name, s))
    } else {
        Ok(val)
    }
}

/// C-compatible main exported for .so loading
#[no_mangle]
pub unsafe extern "C" fn main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int {
    if argc != 3 {
        let prog = if argc > 0 {
            CStr::from_ptr(*argv).to_str().unwrap_or("driver")
        } else {
            "driver"
        };
        eprintln!("Usage: {} base exponent", prog);
        return 1;
    }

    let arg1 = CStr::from_ptr(*argv.offset(1)).to_str().unwrap();
    let arg2 = CStr::from_ptr(*argv.offset(2)).to_str().unwrap();

    let base = match parse_double(arg1, "base") {
        Ok(v) => v,
        Err(e) => { eprintln!("{}", e); return 1; }
    };
    let exponent = match parse_double(arg2, "exponent") {
        Ok(v) => v,
        Err(e) => { eprintln!("{}", e); return 1; }
    };

    *libc::__errno_location() = 0;
    let result = pow(base, exponent);
    let err = *libc::__errno_location();
    if err == libc::EDOM {
        eprintln!("Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.", base, exponent);
        return 1;
    } else if err == libc::ERANGE {
        eprintln!("Range error: pow({:.2}, {:.2}) caused overflow or underflow.", base, exponent);
        return 1;
    }

    println!("Result: {:.2}", result);
    0
}
