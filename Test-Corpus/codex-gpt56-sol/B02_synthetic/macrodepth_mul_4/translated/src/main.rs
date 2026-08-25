mod config;
mod core;

use std::ffi::{CString, c_char, c_int};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;

unsafe extern "C" {
    fn atoi(value: *const c_char) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() < 3 {
        let mut stderr = io::stderr().lock();
        let _ = stderr.write_all(b"usage: ");
        let _ = stderr.write_all(args.first().map_or(b"", |arg| arg.as_os_str().as_bytes()));
        let _ = stderr.write_all(b" A B\n");
        std::process::exit(2);
    }

    let a_arg = CString::new(args[1].as_os_str().as_bytes()).unwrap();
    let b_arg = CString::new(args[2].as_os_str().as_bytes()).unwrap();
    let a = unsafe { atoi(a_arg.as_ptr()) };
    let b = unsafe { atoi(b_arg.as_ptr()) };

    let r_call = core::selected_call(a, b);
    let acc = core::configured_accumulator();

    let x1 = core::helper_call(a, b);
    let x2 = core::helper_ptr(a, b);
    let x3 = core::use_generated(config::REPEAT);
    let g = unsafe { (core::G_OP)(a, b) };
    let op_name = unsafe { core::G_OP_NAME };

    unsafe {
        printf(
            c"op=%s call=%d acc=%d g.call=%d\n".as_ptr(),
            op_name,
            r_call,
            acc,
            g,
        );
    }

    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    unsafe {
        printf(c"summary=%d\n".as_ptr(), summary);
    }
}
