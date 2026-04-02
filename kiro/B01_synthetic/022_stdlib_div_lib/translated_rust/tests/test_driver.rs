use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // Flush before redirecting
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1) };
    unsafe { libc::close(pipe_fds[1]) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(saved_stdout, 1) };
    unsafe { libc::close(saved_stdout) };

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

use std::os::unix::io::FromRawFd;

#[test]
fn test_driver_matches_c() {
    let c_lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    let c_lib = unsafe { Library::new(&c_lib_path).expect("Failed to load C libdriver.so") };
    let c_driver: Symbol<unsafe extern "C" fn(c_int, c_int)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find C driver symbol") };

    let test_cases: &[(c_int, c_int)] = &[
        (10, 3),
        (7, 2),
        (-10, 3),
        (10, -3),
        (-10, -3),
        (0, 1),
        (100, 7),
        (1, 1),
        (i32::MAX, 2),
        (i32::MIN + 1, 2),
    ];

    for &(x, y) in test_cases {
        let c_output = capture_stdout(|| unsafe { c_driver(x, y) });
        let rust_output = capture_stdout(|| driver::driver(x, y));
        assert_eq!(
            c_output, rust_output,
            "Mismatch for driver({}, {}): C={:?}, Rust={:?}",
            x, y, c_output, rust_output
        );
    }
}
