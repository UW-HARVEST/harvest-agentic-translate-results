use std::env;
use std::ffi::{c_char, c_int, c_long, CString};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

unsafe extern "C" {
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
}

fn static_sum(update: i32) -> i32 {
    SUM.fetch_add(update, Ordering::Relaxed)
        .wrapping_add(update)
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();
    let stdout = io::stdout();
    let mut output = stdout.lock();

    if args.len() != 2 {
        let _ = writeln!(output, "Error: should only be a single (integer) argument!");
        return ExitCode::FAILURE;
    }

    let argument =
        CString::new(args[1].as_os_str().as_bytes()).expect("argv cannot contain a NUL byte");
    let start = argument.as_ptr();
    let mut end = std::ptr::null_mut();

    // SAFETY: `argument` is NUL-terminated and remains alive for the call.
    let parsed = unsafe { strtol(start, &mut end, 10) };
    if end == start.cast_mut() {
        let _ = writeln!(output, "Error: first argument must be an integer!");
        return ExitCode::FAILURE;
    }

    let stride = parsed as i32;
    for i in 0_i32..10 {
        let _ = writeln!(output, "{}", static_sum(i.wrapping_mul(stride)));
    }

    ExitCode::SUCCESS
}
