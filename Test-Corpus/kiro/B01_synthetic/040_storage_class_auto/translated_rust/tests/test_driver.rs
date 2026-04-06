use libloading::{Library, Symbol};
use std::os::unix::io::FromRawFd;
use std::io::{Read, Write};

/// Capture stdout from a closure by redirecting fd 1 to a pipe
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // flush both C and Rust stdout before redirect
    std::io::stdout().flush().ok();
    unsafe { libc::fflush(std::ptr::null_mut()); }

    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipes[1], 1); }
    unsafe { libc::close(pipes[1]); }

    f();

    // flush both C and Rust stdout after call
    std::io::stdout().flush().ok();
    unsafe { libc::fflush(std::ptr::null_mut()); }

    // restore stdout
    unsafe { libc::dup2(old_stdout, 1); }
    unsafe { libc::close(old_stdout); }

    let mut buf = String::new();
    let mut read_end = unsafe { std::fs::File::from_raw_fd(pipes[0]) };
    read_end.read_to_string(&mut buf).unwrap();
    buf
}

#[test]
fn test_driver_matches() {
    let c_lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    let c_lib = unsafe { Library::new(&c_lib_path).expect("Failed to load C .so") };
    let c_driver: Symbol<unsafe extern "C" fn(i32)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find driver in C .so") };

    let test_values: &[i32] = &[0, 1, -1, 100, -100, i32::MAX, i32::MIN, 42, 999999];

    for &x in test_values {
        let c_out = capture_stdout(|| unsafe { c_driver(x) });
        let rust_out = capture_stdout(|| driver::driver(x));
        assert_eq!(c_out, rust_out, "Mismatch for driver({}): C={:?} Rust={:?}", x, c_out, rust_out);
    }
}
