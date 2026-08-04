use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout output from a closure that calls into a shared library.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        // flush both C and Rust stdout
        libc::fflush(std::ptr::null_mut()); // flushes all C streams
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        // flush after the call
        libc::fflush(std::ptr::null_mut());

        // restore stdout
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        let mut file = std::fs::File::from_raw_fd(pipe_fds[0]);
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib_path() -> String {
    std::env::current_dir()
        .unwrap()
        .join("c_src/build/libdriver.so")
        .to_str()
        .unwrap()
        .to_string()
}

fn rust_lib_path() -> String {
    std::env::current_dir()
        .unwrap()
        .join("target/debug/libdriver.so")
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_driver_outputs_match() {
    let test_values: &[f32] = &[
        0.0, -0.0, 1.0, -1.0,
        f32::INFINITY, f32::NEG_INFINITY, f32::NAN,
        0.1, 3.14, f32::MIN, f32::MAX, f32::MIN_POSITIVE,
        1e-38, 1e38, 42.0, -273.15,
    ];

    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust .so");

        let c_driver: Symbol<unsafe extern "C" fn(f32)> =
            c_lib.get(b"driver").expect("C driver symbol not found");
        let rust_driver: Symbol<unsafe extern "C" fn(f32)> =
            rust_lib.get(b"driver").expect("Rust driver symbol not found");

        for &val in test_values {
            let c_out = capture_stdout(|| { c_driver(val); });
            let r_out = capture_stdout(|| { rust_driver(val); });
            assert_eq!(
                c_out, r_out,
                "Mismatch for input {val}: C={c_out:?} Rust={r_out:?}"
            );
        }
    }
}
