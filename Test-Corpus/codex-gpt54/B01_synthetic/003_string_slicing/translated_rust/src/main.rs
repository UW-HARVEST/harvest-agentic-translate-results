use std::ffi::{CString, OsString};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::ptr;

fn write_all(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes).unwrap();
}

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    let argc = args.len();

    if argc > 4 || argc == 1 {
        write_all(b"Error: there should be one to three arguments passed:\n");
        write_all(b"<string> [start] [stop]\n");
        std::process::exit(1);
    }

    let string = args[1].as_os_str().as_bytes();
    let len = string.len();

    let start: libc::c_int;
    let stop: libc::c_int;
    let mut end: *mut libc::c_char = ptr::null_mut();

    if argc >= 3 {
        let arg2 = CString::new(args[2].as_os_str().as_bytes()).unwrap();
        let start_long = unsafe { libc::strtol(arg2.as_ptr(), &mut end, 10) };
        start = start_long as libc::c_int;

        if end == arg2.as_ptr() as *mut libc::c_char {
            write_all(b"Second argument must be an integer!");
            std::process::exit(1);
        }

        if (start as usize) > len {
            write_all(b"Error: start is off the end of the string!\n");
            std::process::exit(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        let arg3 = CString::new(args[3].as_os_str().as_bytes()).unwrap();
        let stop_long = unsafe { libc::strtol(arg3.as_ptr(), ptr::null_mut(), 10) };
        stop = stop_long as libc::c_int;

        if end == arg3.as_ptr() as *mut libc::c_char {
            write_all(b"Third argument must be an integer!");
            std::process::exit(1);
        }

        if (stop as usize) > len {
            write_all(b"Error: stop is off the end of the string!\n");
            std::process::exit(1);
        }

        if stop <= start {
            write_all(b"Error: stop must come after start!\n");
            std::process::exit(1);
        }
    } else {
        stop = len as libc::c_int;
    }

    let start_usize = start as usize;
    let stop_usize = stop as usize;
    write_all(&string[start_usize..stop_usize]);
    write_all(b"\n");
}
