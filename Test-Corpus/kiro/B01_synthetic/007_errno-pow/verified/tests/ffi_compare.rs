use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB: &str = env!("C_LIB_PATH");
const RUST_LIB: &str = env!("RUST_LIB_PATH");

type MainFn = unsafe extern "C" fn(libc::c_int, *const *const libc::c_char) -> libc::c_int;

/// Capture stdout and stderr from calling main(argc, argv) in a loaded .so.
/// We redirect fd 1 and fd 2 to pipes, call main, then read the pipes.
fn call_main_capture(lib: &Library, args: &[&str]) -> (i32, String, String) {
    let cstrings: Vec<CString> = args.iter().map(|s| CString::new(*s).unwrap()).collect();
    let ptrs: Vec<*const libc::c_char> = cstrings.iter().map(|c| c.as_ptr()).collect();

    // Create pipes for stdout and stderr
    let mut stdout_pipe = [0i32; 2];
    let mut stderr_pipe = [0i32; 2];
    unsafe {
        libc::pipe(stdout_pipe.as_mut_ptr());
        libc::pipe(stderr_pipe.as_mut_ptr());
    }

    // Save original fds
    let orig_stdout = unsafe { libc::dup(1) };
    let orig_stderr = unsafe { libc::dup(2) };

    // Redirect stdout/stderr to pipes
    unsafe {
        libc::dup2(stdout_pipe[1], 1);
        libc::dup2(stderr_pipe[1], 2);
        libc::close(stdout_pipe[1]);
        libc::close(stderr_pipe[1]);
    }

    // Call main
    let main_fn: Symbol<MainFn> = unsafe { lib.get(b"main").unwrap() };
    let rc = unsafe { main_fn(ptrs.len() as libc::c_int, ptrs.as_ptr()) };

    // Flush C stdio buffers
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Restore original fds
    unsafe {
        libc::dup2(orig_stdout, 1);
        libc::dup2(orig_stderr, 2);
        libc::close(orig_stdout);
        libc::close(orig_stderr);
    }

    // Read captured output
    let mut stdout_str = String::new();
    let mut stderr_str = String::new();
    unsafe {
        let mut f = std::fs::File::from_raw_fd(stdout_pipe[0]);
        f.read_to_string(&mut stdout_str).ok();
        let mut f = std::fs::File::from_raw_fd(stderr_pipe[0]);
        f.read_to_string(&mut stderr_str).ok();
    }

    (rc, stdout_str, stderr_str)
}

fn compare(args: &[&str], c_lib: &Library, r_lib: &Library) {
    let (c_rc, c_out, c_err) = call_main_capture(c_lib, args);
    let (r_rc, r_out, r_err) = call_main_capture(r_lib, args);

    // Normalize program name in usage messages
    let c_err_norm = c_err.replace(args[0], "PROG");
    let r_err_norm = r_err.replace(args[0], "PROG");

    assert_eq!(c_rc, r_rc, "Return code mismatch for args {:?}: C={} Rust={}", args, c_rc, r_rc);
    assert_eq!(c_out, r_out, "Stdout mismatch for args {:?}:\nC:    {:?}\nRust: {:?}", args, c_out, r_out);
    assert_eq!(c_err_norm, r_err_norm, "Stderr mismatch for args {:?}:\nC:    {:?}\nRust: {:?}", args, c_err_norm, r_err_norm);
}

#[test]
fn test_main_via_so() {
    let c_lib = unsafe { Library::new(C_LIB).expect("Failed to load C .so") };
    let r_lib = unsafe { Library::new(RUST_LIB).expect("Failed to load Rust .so") };

    // Normal cases
    compare(&["prog", "2", "3"], &c_lib, &r_lib);
    compare(&["prog", "2", "0"], &c_lib, &r_lib);
    compare(&["prog", "0", "0"], &c_lib, &r_lib);
    compare(&["prog", "0", "5"], &c_lib, &r_lib);
    compare(&["prog", "-2", "3"], &c_lib, &r_lib);
    compare(&["prog", "-2", "2"], &c_lib, &r_lib);
    compare(&["prog", "2.5", "3.5"], &c_lib, &r_lib);
    compare(&["prog", "1", "1000000"], &c_lib, &r_lib);
    compare(&["prog", "2", "-3"], &c_lib, &r_lib);

    // Error cases - domain/range
    compare(&["prog", "10", "309"], &c_lib, &r_lib);
    compare(&["prog", "-1", "0.5"], &c_lib, &r_lib);
    compare(&["prog", "0", "-1"], &c_lib, &r_lib);
    compare(&["prog", "1e308", "2"], &c_lib, &r_lib);

    // Invalid input
    compare(&["prog", "abc", "def"], &c_lib, &r_lib);
    compare(&["prog", "2", "abc"], &c_lib, &r_lib);
    compare(&["prog", "abc", "2"], &c_lib, &r_lib);

    // Wrong argc
    compare(&["prog"], &c_lib, &r_lib);
    compare(&["prog", "2", "3", "4"], &c_lib, &r_lib);
}
