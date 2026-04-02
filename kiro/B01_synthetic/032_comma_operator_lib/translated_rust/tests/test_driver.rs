use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout from a closure that writes to fd 1 (C printf or Rust println).
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    unsafe {
        // fflush(NULL) flushes all open streams
        libc::fflush(std::ptr::null_mut());

        let mut pipes = [0i32; 2];
        assert_eq!(libc::pipe(pipes.as_mut_ptr()), 0);

        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);

        libc::dup2(pipes[1], 1);
        libc::close(pipes[1]);

        f();

        std::io::stdout().flush().unwrap();
        libc::fflush(std::ptr::null_mut());

        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut buf = String::new();
        let mut read_pipe = std::fs::File::from_raw_fd(pipes[0]);
        read_pipe.read_to_string(&mut buf).unwrap();
        buf
    }
}

#[test]
fn test_driver_output_matches() {
    let c_lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");

    let c_lib = unsafe { libloading::Library::new(&c_lib_path).expect("Failed to load C .so") };
    let c_driver: libloading::Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find C driver symbol") };

    for x in [0, 1, 5, 10] {
        let c_output = capture_stdout(|| unsafe { c_driver(x) });
        let rust_output = capture_stdout(|| driver::driver(x));

        assert_eq!(
            c_output, rust_output,
            "Mismatch for driver({})\nC output:\n{}\nRust output:\n{}",
            x, c_output, rust_output
        );
    }
}
