// Integration test: compares C .so and Rust .so behavior of `driver(float)`.
// The function prints the raw bytes of a float in hex to stdout.
// We capture stdout via a pipe & dup2 and compare byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_float;
use std::io::Read;
use std::os::unix::io::RawFd;
use std::path::PathBuf;

type DriverFn = unsafe extern "C" fn(c_float);

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built into target/<profile>/libdriver.so when cargo
    // builds for tests as well (tests link an rlib but the cdylib is
    // produced because of crate-type = ["cdylib"]).
    let mut candidates = vec![
        project_root().join("target/debug/libdriver.so"),
        project_root().join("target/release/libdriver.so"),
    ];
    candidates.retain(|p| p.exists());
    candidates
        .into_iter()
        .next()
        .expect("Rust libdriver.so not found; run `cargo build` first")
}

use std::sync::Mutex;

// Global mutex: only one test may capture stdout at a time.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

extern "C" {
    fn fflush(stream: *mut libc::FILE) -> i32;
}

/// Capture everything written to stdout (fd 1) while running `f`.
/// Uses pipe + dup2 on file descriptor 1. NOT thread-safe — uses a global
/// lock to serialize.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Flush all FILE* streams (Rust's stdout has nothing meaningful
        // since the test harness captures it, but be safe).
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        fflush(std::ptr::null_mut()); // flush all libc streams

        // Save original stdout fd.
        let saved: RawFd = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        // Create pipe.
        let mut fds: [libc::c_int; 2] = [0, 0];
        let r = libc::pipe(fds.as_mut_ptr());
        assert_eq!(r, 0, "pipe() failed");
        let (read_fd, write_fd) = (fds[0], fds[1]);

        // Redirect stdout to write end of pipe.
        let r = libc::dup2(write_fd, 1);
        assert!(r >= 0, "dup2 failed");
        libc::close(write_fd);

        // Run the function (any printf inside writes to fd 1 = pipe).
        f();

        // Flush all libc FILE* streams to fd 1 (the pipe).
        fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        // Restore original stdout — this closes the pipe write side
        // (which was at fd 1), so the read end will see EOF.
        let r = libc::dup2(saved, 1);
        assert!(r >= 0, "restore dup2 failed");
        libc::close(saved);

        // Read everything from pipe read end until EOF.
        use std::os::unix::io::FromRawFd;
        let mut buf = Vec::new();
        let mut file = std::fs::File::from_raw_fd(read_fd);
        file.read_to_end(&mut buf).expect("failed to read pipe");
        buf
    }
}

fn run_driver(lib_path: &PathBuf, x: f32) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let lib = Library::new(lib_path)
            .unwrap_or_else(|e| panic!("failed to load {}: {}", lib_path.display(), e));
        let f: Symbol<DriverFn> = lib
            .get(b"driver")
            .expect("symbol `driver` not found");
        f(x);
        // lib drops here; on Linux this typically unloads.
    })
}

fn check(x: f32) {
    let c = run_driver(&c_lib_path(), x);
    let r = run_driver(&rust_lib_path(), x);
    assert_eq!(
        c, r,
        "mismatch for input {x:?} (bits=0x{:08x})\n  C   = {:?}\n  Rust= {:?}",
        x.to_bits(),
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r),
    );
}

#[test]
fn driver_zero() {
    check(0.0_f32);
}

#[test]
fn driver_neg_zero() {
    check(-0.0_f32);
}

#[test]
fn driver_one() {
    check(1.0_f32);
}

#[test]
fn driver_neg_one() {
    check(-1.0_f32);
}

#[test]
fn driver_pi() {
    check(std::f32::consts::PI);
}

#[test]
fn driver_e() {
    check(std::f32::consts::E);
}

#[test]
fn driver_nan() {
    check(f32::NAN);
}

#[test]
fn driver_inf() {
    check(f32::INFINITY);
}

#[test]
fn driver_neg_inf() {
    check(f32::NEG_INFINITY);
}

#[test]
fn driver_min_positive() {
    check(f32::MIN_POSITIVE);
}

#[test]
fn driver_max() {
    check(f32::MAX);
}

#[test]
fn driver_min() {
    check(f32::MIN);
}

#[test]
fn driver_subnormal() {
    check(f32::from_bits(0x0000_0001));
}

#[test]
fn driver_random_bits_sweep() {
    // Deterministic sweep across many representative bit patterns.
    let bit_patterns: &[u32] = &[
        0x0000_0000,
        0x8000_0000,
        0x3F80_0000, // 1.0
        0xBF80_0000, // -1.0
        0x4048_F5C3, // ~3.14
        0x402D_F854, // ~e
        0x7F80_0000, // +inf
        0xFF80_0000, // -inf
        0x7FC0_0000, // qnan
        0xFFC0_0000, // qnan negative
        0x7F7F_FFFF, // f32::MAX
        0xFF7F_FFFF, // f32::MIN
        0x0080_0000, // f32::MIN_POSITIVE
        0x0000_0001, // smallest subnormal
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x1234_5678,
        0x89AB_CDEF,
    ];
    for &bits in bit_patterns {
        check(f32::from_bits(bits));
    }
}
