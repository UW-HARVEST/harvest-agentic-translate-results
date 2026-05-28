use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::PathBuf;

type DriverFn = unsafe extern "C" fn(c_int);

extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

/// Capture everything written to fd 1 (stdout) by the closure.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush any pending output first.
        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup failed");

        let mut fds = [0i32; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let read_fd = fds[0];
        let write_fd = fds[1];

        assert!(dup2(write_fd, 1) >= 0, "dup2 failed");
        close(write_fd);

        // Run the closure that writes to stdout.
        f();

        // Flush both C stdio and Rust stdout buffers
        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // Restore stdout and close the duplicate
        dup2(saved_stdout, 1);
        close(saved_stdout);

        // Read all data from the pipe
        let mut file = std::fs::File::from_raw_fd(read_fd);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read pipe failed");
        // file's drop closes read_fd
        let _ = file.into_raw_fd();
        close(read_fd);

        buf
    }
}

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libdriver.so");
    p
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/release/libdriver.so");
    if !p.exists() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("target/debug/libdriver.so");
    }
    p
}

fn run_driver(lib_path: &PathBuf, x: c_int) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("failed to load lib");
        let driver: Symbol<DriverFn> = lib.get(b"driver").expect("no driver symbol");
        let out = capture_stdout(|| driver(x));
        drop(driver);
        drop(lib);
        out
    }
}

#[test]
fn compare_driver_outputs() {
    let c_path = c_lib_path();
    let rust_path = rust_lib_path();

    assert!(c_path.exists(), "C lib not built: {:?}", c_path);
    assert!(rust_path.exists(), "Rust lib not built: {:?}", rust_path);

    let test_inputs: &[c_int] = &[
        0, 1, -1, 2, -2, 100, -100, 12345, -12345,
        i32::MAX / 4, i32::MIN / 4,
        // Edge cases that don't overflow signed 32-bit when doubled and added 300
        (i32::MAX - 300) / 2,
        (i32::MIN + 300) / 2 + 1,
    ];

    for &x in test_inputs {
        let c_out = run_driver(&c_path, x);
        let rust_out = run_driver(&rust_path, x);
        assert_eq!(
            c_out, rust_out,
            "mismatch for x={}: C={:?} Rust={:?}",
            x,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );
    }
}
