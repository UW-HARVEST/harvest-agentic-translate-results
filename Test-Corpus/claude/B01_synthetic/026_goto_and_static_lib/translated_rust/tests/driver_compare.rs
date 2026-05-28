// Integration test: load both the C-built libdriver.so and the Rust-built
// libdriver.so via libloading, invoke `driver(x, y, z)` on each while
// capturing stdout, and assert the captured bytes match exactly.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;

type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, the cdylib in
    // target/<profile>/. CARGO_MANIFEST_DIR + target/debug/libdriver.so
    // is the canonical location after `cargo test` builds the cdylib.
    let mut candidates = Vec::new();
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in &["debug", "release"] {
        candidates.push(base.join(profile).join("libdriver.so"));
    }
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Could not find Rust libdriver.so in any of: {:?}", candidates);
}

/// Capture everything written to fd 1 (stdout) by the closure.
/// Uses a pipe + dup2 so it captures C printf output too, not just
/// Rust's println!.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush libc stdout buffers before swapping the fd, so that any
        // previously-buffered bytes go to the real terminal, not our pipe.
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup failed");

        let mut fds: [c_int; 2] = [0, 0];
        let r = libc::pipe(fds.as_mut_ptr());
        assert_eq!(r, 0, "pipe failed");
        let read_fd = fds[0];
        let write_fd = fds[1];

        let r = libc::dup2(write_fd, 1);
        assert!(r >= 0, "dup2 failed");
        libc::close(write_fd);

        // Run user code.
        f();

        // Flush libc stdout so all printf output reaches the pipe before
        // we restore the original fd.
        libc::fflush(std::ptr::null_mut());

        // Restore original stdout.
        libc::dup2(saved, 1);
        libc::close(saved);

        // Read everything from the pipe.
        let mut file = std::fs::File::from_raw_fd(read_fd);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read pipe");
        buf
    }
}

fn run_case(x: c_int, y: c_int, z: c_int) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_driver: Symbol<DriverFn> = unsafe { c_lib.get(b"driver").expect("c driver") };
    let rust_driver: Symbol<DriverFn> = unsafe { rust_lib.get(b"driver").expect("rust driver") };

    let c_out = capture_stdout(|| unsafe { c_driver(x, y, z) });
    let rust_out = capture_stdout(|| unsafe { rust_driver(x, y, z) });

    assert_eq!(
        c_out,
        rust_out,
        "Mismatch for driver({x}, {y}, {z})\nC output:\n{}\nRust output:\n{}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
    );
}

#[test]
fn driver_x_not_one() {
    run_case(0, 2, 3);
    run_case(2, 2, 3);
    run_case(-5, 2, 3);
}

#[test]
fn driver_y_not_two() {
    run_case(1, 0, 3);
    run_case(1, 1, 3);
    run_case(1, 123, 3);
    run_case(1, -2, 3);
}

#[test]
fn driver_z_not_three() {
    run_case(1, 2, 0);
    run_case(1, 2, 4);
    run_case(1, 2, -3);
}

#[test]
fn driver_ok() {
    run_case(1, 2, 3);
}

#[test]
fn driver_repeated_calls() {
    // y is a static; ensure both libs maintain state identically across calls.
    run_case(1, 2, 3);
    run_case(1, 5, 3); // expect y != 2 path
    run_case(1, 2, 3); // back to ok
    run_case(0, 2, 3); // x != 1 path
}
