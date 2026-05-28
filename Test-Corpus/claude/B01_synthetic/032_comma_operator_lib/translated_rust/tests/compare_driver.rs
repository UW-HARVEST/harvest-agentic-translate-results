use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB: &str = "c_src/build/libdriver.so";
const RUST_LIB: &str = "target/debug/libdriver.so";

/// Capture everything written to stdout while `f` runs and return it as bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush any pending output before redirecting.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Save original stdout fd.
    let stdout_fd = 1;
    let saved_fd = unsafe { libc::dup(stdout_fd) };
    assert!(saved_fd >= 0, "dup failed");

    // Create a pipe.
    let mut fds = [0i32; 2];
    let r = unsafe { libc::pipe(fds.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe failed");
    let read_fd = fds[0];
    let write_fd = fds[1];

    // Redirect stdout to pipe write end.
    let r = unsafe { libc::dup2(write_fd, stdout_fd) };
    assert!(r >= 0, "dup2 failed");
    unsafe {
        libc::close(write_fd);
    }

    // Run the closure.
    f();

    // Flush before restoring.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Restore stdout.
    let r = unsafe { libc::dup2(saved_fd, stdout_fd) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe {
        libc::close(saved_fd);
    }

    // Read pipe.
    let mut file = unsafe { File::from_raw_fd(read_fd) };
    let mut out = Vec::new();
    file.read_to_end(&mut out).expect("read pipe");
    out
}

unsafe fn call_driver(lib: &Library, x: c_int) -> Vec<u8> {
    let func: Symbol<unsafe extern "C" fn(c_int)> =
        lib.get(b"driver\0").expect("driver symbol");
    capture_stdout(|| {
        func(x);
    })
}

fn open_libs() -> (Library, Library) {
    let c = unsafe { Library::new(C_LIB) }.expect("load C lib");
    let r = unsafe { Library::new(RUST_LIB) }.expect("load Rust lib");
    (c, r)
}

#[test]
fn driver_matches_for_various_inputs() {
    let (c_lib, rust_lib) = open_libs();
    let test_inputs: &[c_int] = &[
        -5, -1, 0, 1, 2, 5, 10, 100, 1000,
    ];
    for &x in test_inputs {
        let c_out = unsafe { call_driver(&c_lib, x) };
        let r_out = unsafe { call_driver(&rust_lib, x) };
        assert_eq!(
            c_out,
            r_out,
            "Mismatch for x = {}: C={:?}, Rust={:?}",
            x,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}
