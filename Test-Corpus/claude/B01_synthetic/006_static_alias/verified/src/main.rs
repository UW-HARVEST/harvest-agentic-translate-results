// `driver` executable: the counterpart of the program produced by
// c_src/CMakeLists.txt (`add_executable(driver src/main.c)`).
//
// It rebuilds the `argc` / `argv` pair exactly the way the C runtime hands it to
// `main()` and then calls the translated `main()` (`driver::c_main`), exiting
// with its return value.

use std::ffi::{c_char, c_int, CString, OsString};

/// Bytes of a command line argument, as the C `argv` array would see them.
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned().into_bytes()
    }
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, while a C
/// program starts with the default disposition. Without restoring it, writing to
/// a closed pipe (`driver 1 100000 | head -2`) would make the C program die from
/// `SIGPIPE` (status 141) whereas the Rust one would ignore the failed `write`
/// and exit 0. Restore the C behaviour before any output is produced.
#[cfg(unix)]
fn restore_default_sigpipe() {
    extern "C" {
        fn signal(signum: c_int, handler: usize) -> usize;
    }
    const SIGPIPE: c_int = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    let args: Vec<CString> = std::env::args_os()
        .map(|arg| {
            let mut bytes = arg_bytes(&arg);
            // A C string ends at the first NUL byte; execve() cannot deliver
            // interior NULs, this only guards the conversion.
            if let Some(pos) = bytes.iter().position(|&b| b == 0) {
                bytes.truncate(pos);
            }
            CString::new(bytes).expect("no interior NUL after truncation")
        })
        .collect();

    let mut argv: Vec<*mut c_char> = args.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    argv.push(std::ptr::null_mut());

    let status = unsafe { driver::c_main(args.len() as c_int, argv.as_mut_ptr()) };
    std::process::exit(status);
}
