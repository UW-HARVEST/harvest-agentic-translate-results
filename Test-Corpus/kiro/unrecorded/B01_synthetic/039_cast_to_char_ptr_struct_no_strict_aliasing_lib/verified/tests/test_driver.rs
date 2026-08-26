use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by calling `f`, returns the bytes written.
fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    // flush any pending stdout
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipe_fds = [0 as c_int; 2];
    assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { libc::dup(1) };
    assert!(saved_stdout >= 0);
    unsafe { libc::dup2(pipe_fds[1], 1) };
    unsafe { libc::close(pipe_fds[1]) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(saved_stdout, 1) };
    unsafe { libc::close(saved_stdout) };

    let mut buf = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(&path).expect("failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in the target directory
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let so = manifest.join("target/debug/libdriver.so");
    unsafe { Library::new(&so).expect("failed to load Rust .so") }
}

#[test]
fn test_driver_matches() {
    let c = c_lib();
    let r = rust_lib();

    let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c.get(b"driver").unwrap() };
    let r_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { r.get(b"driver").unwrap() };

    // Test a range of inputs including edge cases
    let test_values: &[c_int] = &[0, 1, -1, 42, i32::MAX, i32::MIN, 100, 255];

    for &val in test_values {
        let c_out = capture_stdout(|| unsafe { c_driver(val) });
        let r_out = capture_stdout(|| unsafe { r_driver(val) });
        assert_eq!(
            c_out, r_out,
            "Mismatch for driver({}): C={:?} Rust={:?}",
            val,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
