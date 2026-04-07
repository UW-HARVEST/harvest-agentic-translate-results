use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // Flush before redirecting
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };

    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1) };
    unsafe { libc::close(pipe_fds[1]) };

    f();

    // Flush C and Rust stdout
    unsafe { libc::fflush(std::ptr::null_mut()) };
    use std::io::Write;
    let _ = std::io::stdout().flush();

    // Restore stdout
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut buf = String::new();
    let mut read_end = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    // Set non-blocking to avoid hanging, then read available
    unsafe {
        let flags = libc::fcntl(pipe_fds[0], libc::F_GETFL);
        libc::fcntl(pipe_fds[0], libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let _ = read_end.read_to_string(&mut buf);
    buf
}

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_build/libdriver_c.so", manifest)
}

fn rust_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/target/debug/libdriver.so", manifest)
}

#[test]
fn test_driver_various_inputs() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let test_values: &[i32] = &[0, 1, -1, 42, 100, i32::MAX, i32::MIN, 999999];

    for &val in test_values {
        let c_output = {
            let func: Symbol<unsafe extern "C" fn(i32)> =
                unsafe { c_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { func(val) })
        };

        let rust_output = {
            let func: Symbol<unsafe extern "C" fn(i32)> =
                unsafe { rust_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { func(val) })
        };

        assert_eq!(
            c_output, rust_output,
            "driver({}) mismatch:\n  C:    {:?}\n  Rust: {:?}",
            val, c_output, rust_output
        );
    }
}
