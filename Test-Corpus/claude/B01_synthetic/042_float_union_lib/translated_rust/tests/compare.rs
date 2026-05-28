// Integration test: load both the C and Rust shared libraries via libloading
// and compare the captured stdout output of the `driver(double)` function.

use libloading::{Library, Symbol};
use std::ffi::c_double;
use std::io::Read;
use std::os::raw::c_int;

extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut libc::FILE) -> c_int;
}

/// Capture stdout produced while running `f`. Works for both C printf and any
/// other writes to the underlying fd 1.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        // Flush any pending stdout
        fflush(std::ptr::null_mut());

        // Save fd 1
        let saved = dup(1);
        assert!(saved >= 0);

        // Pipe
        let mut fds = [0 as c_int; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);

        // Redirect stdout to pipe write end
        assert!(dup2(fds[1], 1) >= 0);
        close(fds[1]);

        // Run user code
        f();

        // Flush stdout
        fflush(std::ptr::null_mut());

        // Restore stdout
        dup2(saved, 1);
        close(saved);

        // Read all from pipe read-end
        let mut file = std::fs::File::from(std::os::fd::OwnedFd::from_raw_fd(fds[0]));
        let mut s = String::new();
        let _ = file.read_to_string(&mut s);
        s
    }
}

use std::os::fd::FromRawFd;

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> std::path::PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // Use the cdylib produced by cargo build.
    // Tests run with cargo, which builds the lib before the tests, so this
    // exists in target/<profile>/libdriver.so.
    let manifest = manifest_dir();
    // Find the most recent libdriver.so under target.
    for profile in &["debug", "release"] {
        let p = manifest.join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust libdriver.so not found");
}

unsafe fn call_driver(lib: &Library, f: c_double) {
    let func: Symbol<unsafe extern "C" fn(c_double)> = lib.get(b"driver").unwrap();
    func(f);
}

fn check(value: f64) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_out = capture_stdout(|| unsafe { call_driver(&c_lib, value) });
    let r_out = capture_stdout(|| unsafe { call_driver(&rust_lib, value) });

    assert_eq!(
        c_out, r_out,
        "Mismatch for input {:?}:\n  C   : {:?}\n  Rust: {:?}",
        value, c_out, r_out
    );
}

#[test]
fn driver_zero() {
    check(0.0);
}

#[test]
fn driver_neg_zero() {
    check(-0.0);
}

#[test]
fn driver_one() {
    check(1.0);
}

#[test]
fn driver_neg_one() {
    check(-1.0);
}

#[test]
fn driver_pi() {
    check(std::f64::consts::PI);
}

#[test]
fn driver_e() {
    check(std::f64::consts::E);
}

#[test]
fn driver_small() {
    check(1e-10);
}

#[test]
fn driver_large() {
    check(1e20);
}

#[test]
fn driver_subnormal() {
    check(f64::from_bits(1));
}

#[test]
fn driver_min_positive() {
    check(f64::MIN_POSITIVE);
}

#[test]
fn driver_max() {
    check(f64::MAX);
}

#[test]
fn driver_min() {
    check(f64::MIN);
}

#[test]
fn driver_inf() {
    check(f64::INFINITY);
}

#[test]
fn driver_neg_inf() {
    check(f64::NEG_INFINITY);
}

#[test]
fn driver_nan() {
    check(f64::NAN);
}

#[test]
fn driver_half() {
    check(0.5);
}

#[test]
fn driver_negative_pi_over_2() {
    check(-std::f64::consts::FRAC_PI_2);
}

#[test]
fn driver_random_assorted() {
    let values = [
        1234.5678_f64,
        -987.654321,
        3.141592653589793e10,
        2.71828e-5,
        1.0 / 3.0,
        0.1,
        0.2,
        0.3,
        100.0,
        12345.6789,
    ];
    for v in values {
        check(v);
    }
}
