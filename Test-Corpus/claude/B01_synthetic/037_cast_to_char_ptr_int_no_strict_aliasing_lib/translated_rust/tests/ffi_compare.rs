// Integration tests comparing the C and Rust implementations of `driver`
// through their compiled shared libraries.
//
// The `driver` function writes to stdout, so we redirect file descriptor 1
// using dup2/pipe to capture the output for comparison.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::sync::Mutex;

// Tests must serialize because they redirect global stdout fd 1.
static FD_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest_dir)
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // The integration test is built into target/debug/deps/, so target/debug
    // is the parent of the deps directory of the test binary.
    // But CARGO_MANIFEST_DIR + target/debug/libdriver.so is more direct.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest_dir)
        .join("target")
        .join("debug")
        .join("libdriver.so")
}

/// Capture everything written to fd 1 (stdout) by the closure.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush Rust's stdout buffer before redirecting.
    use std::io::Write;
    let _ = std::io::stdout().flush();

    unsafe {
        let mut fds: [c_int; 2] = [0, 0];
        let r = libc::pipe(fds.as_mut_ptr());
        assert_eq!(r, 0, "pipe failed");
        let read_fd = fds[0];
        let write_fd = fds[1];

        // Save original stdout
        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0, "dup failed");

        // Redirect stdout (fd 1) to write end of pipe
        let r = libc::dup2(write_fd, 1);
        assert!(r >= 0, "dup2 failed");
        libc::close(write_fd);

        // Run the function
        f();

        // Flush libc's stdout buffer
        // (the C code uses printf which buffers in libc)
        libc::fflush(std::ptr::null_mut());

        // Restore original stdout
        let r = libc::dup2(saved_stdout, 1);
        assert!(r >= 0, "dup2 restore failed");
        libc::close(saved_stdout);

        // Read all data from the pipe
        // Set pipe read end to non-blocking so we don't hang
        let flags = libc::fcntl(read_fd, libc::F_GETFL, 0);
        libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);

        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            let n = libc::read(read_fd, tmp.as_mut_ptr() as *mut _, tmp.len());
            if n <= 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n as usize]);
        }
        libc::close(read_fd);

        buf
    }
}

type DriverFn = unsafe extern "C" fn(c_int);

fn run_driver_c(x: c_int) -> Vec<u8> {
    let _g = FD_LOCK.lock().unwrap();
    let lib = unsafe { Library::new(c_lib_path()).expect("failed to load C lib") };
    let func: Symbol<DriverFn> = unsafe { lib.get(b"driver\0").expect("failed to find driver") };
    capture_stdout(|| unsafe { func(x) })
}

fn run_driver_rust(x: c_int) -> Vec<u8> {
    let _g = FD_LOCK.lock().unwrap();
    let lib = unsafe { Library::new(rust_lib_path()).expect("failed to load Rust lib") };
    let func: Symbol<DriverFn> = unsafe { lib.get(b"driver\0").expect("failed to find driver") };
    capture_stdout(|| unsafe { func(x) })
}

fn check(x: c_int) {
    let c_out = run_driver_c(x);
    let r_out = run_driver_rust(x);
    assert_eq!(
        c_out,
        r_out,
        "Output mismatch for x = {}\nC:    {:?}\nRust: {:?}",
        x,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

#[test]
fn driver_zero() {
    check(0);
}

#[test]
fn driver_one() {
    check(1);
}

#[test]
fn driver_neg_one() {
    check(-1);
}

#[test]
fn driver_max() {
    check(c_int::MAX);
}

#[test]
fn driver_min() {
    check(c_int::MIN);
}

#[test]
fn driver_assorted() {
    for x in [
        0x12345678i32,
        0x00ff00ff,
        0x7f000001,
        -42,
        42,
        0x80000000u32 as i32,
        0xdeadbeefu32 as i32,
    ] {
        check(x);
    }
}
