use libloading::{Library, Symbol};
use std::io::Read;

/// Capture stdout from C's driver() by redirecting fd 1 to a pipe
fn capture_c_driver(lib: &Library, f: f64) -> String {
    unsafe {
        // Create a pipe
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let read_fd = fds[0];
        let write_fd = fds[1];

        // Save original stdout
        let orig_stdout = libc::dup(1);
        assert!(orig_stdout >= 0);

        // Redirect stdout to write end of pipe
        libc::dup2(write_fd, 1);
        libc::close(write_fd);

        // Call C driver
        let driver: Symbol<unsafe extern "C" fn(f64)> = lib.get(b"driver").unwrap();
        driver(f);

        // Flush C stdout
        libc::fflush(std::ptr::null_mut());

        // Restore stdout
        libc::dup2(orig_stdout, 1);
        libc::close(orig_stdout);

        // Read from pipe
        let mut buf = vec![0u8; 4096];
        let n = libc::read(read_fd, buf.as_mut_ptr() as *mut _, buf.len());
        libc::close(read_fd);

        String::from_utf8_lossy(&buf[..n as usize]).to_string()
    }
}

/// Capture Rust driver() output
fn capture_rust_driver(f: f64) -> String {
    let bits = f.to_bits();
    format!(
        "{:x} {} {}\n",
        bits,
        float_union::format_hex_float(f),
        float_union::format_f4(f)
    )
}

#[test]
fn test_driver_outputs_match() {
    let lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    let lib = unsafe { Library::new(&lib_path).expect("Failed to load C library") };

    let test_values: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        0.1,
        3.14159265358979,
        1e100,
        1e-100,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        1.23456789,
        -42.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        2.2250738585072014e-308, // smallest normal
        5e-324,                  // smallest subnormal
    ];

    for &val in test_values {
        let c_out = capture_c_driver(&lib, val);
        let rust_out = capture_rust_driver(val);
        assert_eq!(
            c_out, rust_out,
            "Mismatch for input {val:?} (bits={:#018x}):\n  C:    {:?}\n  Rust: {:?}",
            val.to_bits(), c_out, rust_out
        );
    }
}
