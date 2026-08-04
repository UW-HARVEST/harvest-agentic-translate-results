// Integration test that loads BOTH the C and Rust shared libraries and
// compares stdout byte-for-byte for the `driver(x, y)` function.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

// Capturing fd 1 globally is not concurrency-safe; serialize.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let base = PathBuf::from(manifest);
    for p in [
        base.join("target/release/libdriver.so"),
        base.join("target/debug/libdriver.so"),
    ] {
        if p.exists() {
            return p;
        }
    }
    panic!("Could not find Rust libdriver.so. Build with `cargo build` first.");
}

/// Capture stdout produced by `f`. Redirect OS fd 1 to a pipe, drain on a
/// background thread to avoid blocking on a full pipe buffer for large output,
/// then restore fd 1.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;
    let _ = std::io::stdout().flush();

    let mut fds: [c_int; 2] = [0, 0];
    let r = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe() failed");
    let read_fd = fds[0];
    let write_fd = fds[1];

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");

    let r = unsafe { dup2(write_fd, 1) };
    assert!(r >= 0, "dup2 failed");
    unsafe { close(write_fd) };

    let handle = std::thread::spawn(move || {
        let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        buf
    });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Flush libc stdout so any buffered printf output is emitted before we
    // restore fd 1.
    unsafe {
        let _ = fflush(std::ptr::null_mut());
    }

    unsafe {
        dup2(saved, 1);
        close(saved);
    }

    let output = handle.join().expect("reader thread panicked");
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
    output
}

type DriverFn = unsafe extern "C" fn(c_int, c_int);

fn run_driver(lib_path: &std::path::Path, x: c_int, y: c_int) -> Vec<u8> {
    let _g = STDOUT_LOCK.lock().unwrap();
    let lib = unsafe { Library::new(lib_path).expect("failed to load .so") };
    let driver: Symbol<DriverFn> =
        unsafe { lib.get(b"driver").expect("driver symbol not found") };
    capture_stdout(|| unsafe {
        driver(x, y);
    })
}

fn assert_match(x: c_int, y: c_int) {
    let c = run_driver(&c_lib_path(), x, y);
    let r = run_driver(&rust_lib_path(), x, y);
    if c != r {
        panic!(
            "driver({}, {}) mismatch:\nC   ({} bytes): {:?}\nRust({} bytes): {:?}",
            x,
            y,
            c.len(),
            String::from_utf8_lossy(&c),
            r.len(),
            String::from_utf8_lossy(&r),
        );
    }
}

#[test]
fn driver_zero_zero() {
    assert_match(0, 0);
}

#[test]
fn driver_zero_y() {
    for y in 1..6 {
        assert_match(0, y);
    }
}

#[test]
fn driver_x_zero() {
    for x in 1..6 {
        assert_match(x, 0);
    }
}

#[test]
fn driver_negative_inputs() {
    // Cases where the outer-loop condition fails immediately (both <= 0)
    // OR x <= 0 with y > 0 (terminates because x is never modified once <=0
    // and y eventually decrements to 0).
    assert_match(-1, -1);
    assert_match(-3, 2);
    assert_match(0, -5);
    assert_match(-100, 0);
}

#[test]
fn driver_special_one_four() {
    // The "skip label1 once" trigger.
    assert_match(1, 4);
}

#[test]
fn driver_small_grid() {
    // Avoid the infinite-loop region: the C function loops forever whenever
    // x > 0 and y < 0 (after enough iterations x decays to 0 while y keeps
    // decrementing past 0). So we restrict to non-negative inputs here.
    for x in 0..7 {
        for y in 0..7 {
            assert_match(x, y);
        }
    }
}

#[test]
fn driver_assorted() {
    let cases = [
        (3, 3),
        (4, 1),
        (5, 5),
        (10, 2),
        (2, 10),
        (1, 1),
        (1, 4),
        (3, 4),
    ];
    for (x, y) in cases {
        assert_match(x, y);
    }
}

#[test]
fn driver_larger() {
    let cases = [
        (15, 7),
        (7, 15),
        (20, 20),
        (1, 100),
        (100, 1),
    ];
    for (x, y) in cases {
        assert_match(x, y);
    }
}
