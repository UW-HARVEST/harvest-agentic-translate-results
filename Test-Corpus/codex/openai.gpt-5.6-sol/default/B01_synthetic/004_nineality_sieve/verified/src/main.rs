use std::ffi::{c_char, c_long, OsString};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

unsafe extern "C" {
    fn strtol(input: *const c_char, end: *mut *mut c_char, base: i32) -> c_long;
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();

    if args.len() != 2 {
        print_stdout(b"Error: should only be a single (integer) argument!\n");
        return ExitCode::FAILURE;
    }

    let mut input = args[1].as_os_str().as_bytes().to_vec();
    input.push(0);

    let start = input.as_ptr().cast::<c_char>();
    let mut end = std::ptr::null_mut();
    // Command-line arguments cannot contain NUL, so `input` is a valid C string.
    let parsed = unsafe { strtol(start, &mut end, 10) };
    if end.cast_const() == start {
        print_stdout(b"Error: first argument must be an integer!\n");
        return ExitCode::FAILURE;
    }

    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut val = parsed as i32;
    loop {
        let _ = writeln!(output, "{val}");
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }

    ExitCode::SUCCESS
}

fn print_stdout(message: &[u8]) {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let _ = output.write_all(message);
}
