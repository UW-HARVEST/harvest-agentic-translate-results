use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by calling `f()` by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    let mut pipe_fds = [0i32; 2];
    unsafe { assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0) };
    let [read_fd, write_fd] = pipe_fds;

    // Save original stdout, replace with write end of pipe
    let orig_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };

    f();

    // Flush C stdout (for the C library's printf)
    unsafe { libc::fflush(std::ptr::null_mut()) };

    // Restore original stdout
    unsafe { libc::dup2(orig_stdout, 1) };
    unsafe { libc::close(orig_stdout) };

    // Read captured output
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(&path).expect("failed to load C library") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    unsafe { Library::new(&path).expect("failed to load Rust library") }
}

#[test]
fn test_driver_outputs_match() {
    let c = c_lib();
    let r = rust_lib();

    let test_inputs: &[c_int] = &[0, 1, -1, 100, -100, i32::MAX / 2, i32::MIN / 2];

    for &x in test_inputs {
        let c_output = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        let r_output = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        assert_eq!(
            c_output, r_output,
            "mismatch for input x={x}: C={c_output:?} Rust={r_output:?}"
        );
    }
}
