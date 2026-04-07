use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // Create a pipe
    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(fds[1], 1); }
    f();
    unsafe {
        libc::fflush(std::ptr::null_mut());
        // Flush Rust stdout too
        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(fds[1]);
    }
    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");
    unsafe { Library::new(dir).expect("failed to load Rust .so") }
}

#[test]
fn test_driver_various_inputs() {
    let c = c_lib();
    let r = rust_lib();

    let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c.get(b"driver").expect("C: missing driver") };
    let r_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { r.get(b"driver").expect("Rust: missing driver") };

    let test_values: &[c_int] = &[
        0, 1, -1, 42, 255, 256, 65535, -65536,
        c_int::MAX, c_int::MIN, 0x12345678, -0x12345678,
    ];

    for &val in test_values {
        let c_out = capture_stdout(|| unsafe { c_driver(val) });
        let r_out = capture_stdout(|| unsafe { r_driver(val) });
        assert_eq!(c_out, r_out, "mismatch for driver({}): C={:?} Rust={:?}", val, c_out, r_out);
    }
}
