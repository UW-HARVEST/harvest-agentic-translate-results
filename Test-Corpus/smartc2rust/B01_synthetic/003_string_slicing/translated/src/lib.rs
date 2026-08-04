
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

use std::io::Write;

fn parse_int_c_like(s: &str) -> (i64, bool) {
    // Mimics strtol behavior enough for our needs:
    // Returns (value, consumed_any_digits).
    // Skips leading whitespace, optional sign, then decimal digits.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let sign: i64 = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        -1
    } else if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
        1
    } else {
        1
    };
    let start_digits = i;
    let mut val: i64 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        val = val.saturating_mul(10).saturating_add(d);
        i += 1;
    }
    let consumed = i > start_digits;
    (sign.saturating_mul(val), consumed)
}

fn run(args: &[String]) -> i32 {
    let argc = args.len();

    if argc > 4 || argc == 1 {
        println!("Error: there should be one to three arguments passed:");
        println!("<string> [start] [stop]");
        return 1;
    }

    let arg1 = &args[1];
    let len: i64 = arg1.len() as i64;

    let start: i64 = if argc >= 3 {
        let (val, ok) = parse_int_c_like(&args[2]);
        if !ok {
            print!("Second argument must be an integer!");
            let _ = std::io::stdout().flush();
            return 1;
        }
        if val > len {
            println!("Error: start is off the end of the string!");
            return 1;
        }
        val
    } else {
        0
    };

    let stop: i64 = if argc == 4 {
        let (val, _ok) = parse_int_c_like(&args[3]);
        // Note: The original C code has a bug where it checks `end == argv[3]`,
        // but `end` was set from parsing argv[2] (or is uninitialized when argc==4
        // with argc<3 which cannot happen here). To preserve observable behavior
        // for typical inputs (numeric third argument), we accept the parse result.
        if val > len {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        if val <= start {
            println!("Error: stop must come after start!");
            return 1;
        }
        val
    } else {
        len
    };

    let s = start as usize;
    let e = stop as usize;
    let slice = &arg1.as_bytes()[s..e];

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(slice);
    let _ = handle.write_all(b"\n");

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    // FFI boundary: convert argv into a safe Vec<String> and delegate to safe Rust.
    let argc = _argc.max(0) as usize;
    let mut args: Vec<String> = Vec::with_capacity(argc);
    for i in 0..argc {
        let p = unsafe { *_argv.add(i) };
        if p.is_null() {
            args.push(String::new());
        } else {
            let cstr = unsafe { std::ffi::CStr::from_ptr(p) };
            args.push(cstr.to_string_lossy().into_owned());
        }
    }
    run(&args) as c_int
}