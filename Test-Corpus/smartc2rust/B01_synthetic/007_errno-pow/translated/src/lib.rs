
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

use std::ffi::CStr;

enum ParseError {
    Range,
    Invalid,
}

fn parse_double(s: &str) -> Result<f64, ParseError> {
    // Mimic strtod: parse leading numeric portion, treat trailing non-numeric as invalid,
    // treat overflow (parsed but infinite while input not literally infinite) as range error.
    let trimmed = s.trim_start();
    match trimmed.parse::<f64>() {
        Ok(v) => {
            if v.is_infinite() {
                let lower = trimmed.to_ascii_lowercase();
                if lower.contains("inf") {
                    Ok(v)
                } else {
                    Err(ParseError::Range)
                }
            } else {
                Ok(v)
            }
        }
        Err(_) => Err(ParseError::Invalid),
    }
}

fn collect_args(argc: c_int, argv: *mut *mut c_char) -> Vec<String> {
    (0..argc)
        .map(|i| {
            let cstr = unsafe { CStr::from_ptr(*argv.offset(i as isize)) };
            cstr.to_string_lossy().into_owned()
        })
        .collect()
}

fn run(args: &[String]) -> c_int {
    if args.len() != 3 {
        let prog = args.first().map(String::as_str).unwrap_or("program");
        eprintln!("Usage: {} base exponent", prog);
        return 1;
    }

    let base = match parse_double(&args[1]) {
        Ok(v) => v,
        Err(ParseError::Range) => {
            eprintln!("Range error while converting base '{}'", args[1]);
            return 1;
        }
        Err(ParseError::Invalid) => {
            eprintln!("Invalid numeric input for base: '{}'", args[1]);
            return 1;
        }
    };

    let exponent = match parse_double(&args[2]) {
        Ok(v) => v,
        Err(ParseError::Range) => {
            eprintln!("Range error while converting exponent '{}'", args[2]);
            return 1;
        }
        Err(ParseError::Invalid) => {
            eprintln!("Invalid numeric input for exponent: '{}'", args[2]);
            return 1;
        }
    };

    let result = base.powf(exponent);

    if result.is_nan() {
        eprintln!(
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        return 1;
    }

    let overflow = result.is_infinite();
    let underflow = result == 0.0 && base != 0.0 && exponent != 0.0;
    if overflow || underflow {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        return 1;
    }

    println!("Result: {:.2}", result);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let args = collect_args(argc, argv);
    run(&args)
}