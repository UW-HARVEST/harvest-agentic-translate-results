use std::env;
use std::ffi::{c_char, c_long, CString};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::ptr;

unsafe extern "C" {
    fn strtol(input: *const c_char, end: *mut *mut c_char, base: i32) -> c_long;
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();

    if args.len() != 2 {
        let _ = io::stdout().write_all(b"Error: should only be a single (integer) argument!\n");
        return ExitCode::FAILURE;
    }

    // Unix process arguments cannot contain an interior NUL.
    let input = CString::new(args[1].as_os_str().as_bytes()).unwrap();
    let mut end = ptr::null_mut();
    let parsed = unsafe { strtol(input.as_ptr(), &mut end, 10) };

    if end == input.as_ptr().cast_mut() {
        let _ = io::stdout().write_all(b"Error: first argument must be an integer!\n");
        return ExitCode::FAILURE;
    }

    let mut val = parsed as i32;
    let mut stdout = io::stdout().lock();
    loop {
        let _ = writeln!(stdout, "{val}");
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }

    ExitCode::SUCCESS
}
