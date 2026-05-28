use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::{FromRawFd, RawFd};

type DriverFn = unsafe extern "C" fn(c_int, c_int);

const C_LIB: &str = "c_src/build/libdriver.so";
const RUST_LIB: &str = "target/release/libdriver.so";

unsafe extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// Capture everything written to stdout (fd 1) by `f`. Returns captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Make sure existing stdio buffers are flushed before redirecting.
    unsafe { fflush(std::ptr::null_mut()) };

    // Save real stdout and replace fd 1 with our pipe writer.
    let saved_stdout: RawFd = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup failed");

    // Build a pipe to capture output.
    let mut fds = [0 as c_int; 2];
    unsafe extern "C" {
        fn pipe(pipefd: *mut c_int) -> c_int;
    }
    let r = unsafe { pipe(fds.as_mut_ptr()) };
    assert!(r == 0, "pipe failed");
    let read_fd = fds[0];
    let write_fd = fds[1];

    // Redirect stdout (fd 1) to write end of the pipe.
    let r = unsafe { dup2(write_fd, 1) };
    assert!(r >= 0, "dup2 failed");
    // Close the write end, since fd 1 is now its alias.
    unsafe { close(write_fd) };

    // Run the function.
    f();

    // Flush any libc buffered output.
    unsafe { fflush(std::ptr::null_mut()) };

    // Restore real stdout.
    let r = unsafe { dup2(saved_stdout, 1) };
    assert!(r >= 0, "restore dup2 failed");
    unsafe { close(saved_stdout) };

    // Read everything from the pipe read end.
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read pipe");
    buf
}

fn run_driver(lib_path: &str, x: c_int, y: c_int) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("load lib");
        let f: Symbol<DriverFn> = lib.get(b"driver").expect("symbol driver");
        capture_stdout(|| f(x, y))
    }
}

#[test]
fn driver_matches_c_for_various_inputs() {
    let cases: &[(c_int, c_int)] = &[
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (-1, 0),
        (0, -1),
        (-1, -1),
        (5, 3),
        (10, 20),
        (i32::MAX, 0),
        (0, i32::MAX),
        (i32::MIN, 0),
        (0, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
        (-7, 12345),
        (12345, -7),
        (0xdeadbeefu32 as i32, 0x1234_5678),
    ];

    for &(x, y) in cases {
        let c_out = run_driver(C_LIB, x, y);
        let r_out = run_driver(RUST_LIB, x, y);
        assert_eq!(
            c_out, r_out,
            "driver({x}, {y}): c={:?} rust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
