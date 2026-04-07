use libloading::{Library, Symbol};

/// Capture stdout output from calling `driver(f)` via the given shared library.
fn call_driver_capture(lib: &Library, f: f64) -> String {
    unsafe {
        // Create a pipe
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let read_fd = fds[0];
        let write_fd = fds[1];

        // Save original stdout
        let orig_stdout = libc::dup(1);
        assert!(orig_stdout >= 0);

        // Redirect stdout to pipe write end
        libc::fflush(std::ptr::null_mut()); // flush any buffered stdout
        assert_eq!(libc::dup2(write_fd, 1), 1);

        // Call driver
        let func: Symbol<unsafe extern "C" fn(f64)> = lib.get(b"driver").unwrap();
        func(f);

        // Flush and restore stdout
        libc::fflush(std::ptr::null_mut());
        libc::dup2(orig_stdout, 1);
        libc::close(orig_stdout);
        libc::close(write_fd);

        // Read captured output
        let mut buf = vec![0u8; 4096];
        let n = libc::read(read_fd, buf.as_mut_ptr() as *mut _, buf.len());
        libc::close(read_fd);

        assert!(n >= 0);
        String::from_utf8_lossy(&buf[..n as usize]).to_string()
    }
}

fn rust_lib_path() -> std::path::PathBuf {
    // The Rust .so is built alongside the test binary
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove 'deps'
    path.push("libdriver.so");
    path
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    let test_values: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        0.5,
        1.5,
        -273.15,
        3.141592653589793,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        1e-300,
        1e300,
        2.2250738585072014e-308, // smallest normal
        5e-324,                  // smallest subnormal
        1.0f64 / 3.0,
        -0.0001,
        999999.9999,
        42.0,
    ];

    for &val in test_values {
        let c_out = call_driver_capture(&c_lib, val);
        let rust_out = call_driver_capture(&rust_lib, val);
        assert_eq!(
            c_out, rust_out,
            "Mismatch for input {val:?}:\n  C:    {c_out:?}\n  Rust: {rust_out:?}"
        );
    }
}
