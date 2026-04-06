use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let (pipe_read, pipe_write) = (pipe_fds[0], pipe_fds[1]);

    let orig_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_write, 1); }
    unsafe { libc::close(pipe_write); }

    f();

    unsafe { libc::fflush(std::ptr::null_mut()); }
    std::io::stdout().flush().unwrap();

    unsafe { libc::dup2(orig_stdout, 1); }
    unsafe { libc::close(orig_stdout); }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_read) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

#[test]
fn test_driver_matches() {
    let c_lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    let c_lib = unsafe { Library::new(&c_lib_path).expect("Failed to load C libdriver.so") };
    let c_driver: Symbol<unsafe extern "C" fn(c_int, c_int)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find 'driver' in C lib") };

    let test_cases: &[(c_int, c_int)] = &[
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (255, 128),
        (0x7FFFFFFF, 0),
        (0, 0x7FFFFFFF),
        (-1, -1),
        (42, 17),
    ];

    for &(x, y) in test_cases {
        let c_output = capture_stdout(|| unsafe { c_driver(x, y) });
        let rust_output = capture_stdout(|| driver::driver(x, y));
        assert_eq!(
            c_output.as_bytes(),
            rust_output.as_bytes(),
            "Mismatch for driver({}, {}): C={:?}, Rust={:?}",
            x, y, c_output, rust_output
        );
    }
}
