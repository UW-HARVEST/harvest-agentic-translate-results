use std::env;
use std::ffi::CString;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

fn main() {
    let args: Vec<_> = env::args_os().collect();
    let mut stdout = io::stdout().lock();

    if args.len() != 2 {
        let _ = stdout.write_all(b"Error: should only be a single (integer) argument!\n");
        std::process::exit(1);
    }

    #[cfg(unix)]
    let arg_bytes = args[1].as_bytes();

    #[cfg(not(unix))]
    let arg_bytes = args[1].to_string_lossy().into_owned().into_bytes();

    let c_arg = match CString::new(arg_bytes) {
        Ok(value) => value,
        Err(_) => {
            let _ = stdout.write_all(b"Error: first argument must be an integer!\n");
            std::process::exit(1);
        }
    };

    let mut end = std::ptr::null_mut();
    let parsed = unsafe { libc::strtol(c_arg.as_ptr(), &mut end, 10) };

    if end == c_arg.as_ptr() as *mut libc::c_char {
        let _ = stdout.write_all(b"Error: first argument must be an integer!\n");
        std::process::exit(1);
    }

    let mut val = parsed as libc::c_int;
    loop {
        let _ = writeln!(stdout, "{val}");
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }
}
