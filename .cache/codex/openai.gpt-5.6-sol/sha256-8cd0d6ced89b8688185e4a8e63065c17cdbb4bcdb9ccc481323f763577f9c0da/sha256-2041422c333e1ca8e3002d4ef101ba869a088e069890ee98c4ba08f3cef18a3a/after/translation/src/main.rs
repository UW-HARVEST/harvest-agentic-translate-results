mod core;

use std::ffi::{CString, c_char, c_int};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn atoi(value: *const c_char) -> c_int;
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments.len() < 3 {
        let mut stderr = io::stderr().lock();
        let _ = stderr.write_all(b"usage: ");
        let _ = stderr.write_all(arguments.first().map_or(b"", |arg| arg.as_bytes()));
        let _ = stderr.write_all(b" A B\n");
        return ExitCode::from(2);
    }

    let a_text = CString::new(arguments[1].as_bytes()).expect("argv cannot contain NUL");
    let b_text = CString::new(arguments[2].as_bytes()).expect("argv cannot contain NUL");
    let a = unsafe { atoi(a_text.as_ptr()) };
    let b = unsafe { atoi(b_text.as_ptr()) };

    let r_call = core::selected_operation(a, b);
    let acc = core::repeated_accumulator(core::REPEAT);

    let x1 = core::helper_call(a, b);
    let x2 = core::helper_ptr(a, b);
    let x3 = core::use_generated(core::REPEAT);
    let g = unsafe { core::G_OP(a, b) };

    unsafe {
        printf(
            c"op=%s call=%d acc=%d g.call=%d\n".as_ptr(),
            core::G_OP_NAME,
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

    ExitCode::SUCCESS
}
