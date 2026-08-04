use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

/// Capture stdout produced by calling `f`, using pipe + dup2.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        // Flush C and Rust stdout before redirecting
        libc::fflush(std::ptr::null_mut());

        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        // Flush after the call
        libc::fflush(std::ptr::null_mut());

        // Restore stdout
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        let mut file = std::fs::File::from_raw_fd(pipe_fds[0]);
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn test_values() -> Vec<f32> {
    vec![
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        0.5,
        -0.5,
        1234.5678,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        1e-38,
        1e38,
    ]
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_driver: Symbol<unsafe extern "C" fn(f32)> =
        unsafe { c_lib.get(b"driver").expect("C driver symbol") };
    let rust_driver: Symbol<unsafe extern "C" fn(f32)> =
        unsafe { rust_lib.get(b"driver").expect("Rust driver symbol") };

    for &val in &test_values() {
        let c_out = capture_stdout(|| unsafe { c_driver(val) });
        let rust_out = capture_stdout(|| unsafe { rust_driver(val) });
        assert_eq!(
            c_out, rust_out,
            "Mismatch for input {val}: C={c_out:?}, Rust={rust_out:?}"
        );
    }
}
