use libloading::{Library, Symbol};
use std::io::Read;

/// Capture stdout produced by `f()` by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // Create a pipe
    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()); }
    let (read_fd, write_fd) = (fds[0], fds[1]);

    // Save original stdout and redirect
    let orig_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1); }

    f();

    // Flush C stdout
    unsafe { libc::fflush(std::ptr::null_mut()); }

    // Restore stdout and close write end
    unsafe { libc::dup2(orig_stdout, 1); }
    unsafe { libc::close(orig_stdout); }
    unsafe { libc::close(write_fd); }

    // Read captured output
    let mut buf = String::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_string(&mut buf).unwrap();
    buf
}

use std::os::unix::io::FromRawFd;

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

#[test]
fn test_driver_matches() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C lib") };
    let c_driver: Symbol<unsafe extern "C" fn(f32)> =
        unsafe { lib.get(b"driver").expect("Failed to find driver symbol") };

    let test_values: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        std::f32::consts::PI,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1.23456789e10,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
    ];

    for &val in test_values {
        let c_out = capture_stdout(|| unsafe { c_driver(val) });
        let rust_out = capture_stdout(|| driver::driver(val));
        assert_eq!(
            c_out, rust_out,
            "Mismatch for input {val}: C={c_out:?} Rust={rust_out:?}"
        );
    }
}
