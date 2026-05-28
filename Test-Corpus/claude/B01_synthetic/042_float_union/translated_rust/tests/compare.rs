// Compares the C shared library's `driver(double)` output against the Rust
// shared library's `driver(double)` output for byte-identical equality.
//
// Both .so files are loaded via libloading; their `driver` symbol is invoked
// through dlsym. We redirect stdout (fd 1) to a temp file around each call so
// we can capture the printf output from libc.
//
// All comparisons are run inside a single `#[test]` so the libtest harness
// does not race with us writing progress messages onto fd 1 (which would be
// caught by our redirect and corrupt the captured bytes).

use libloading::{Library, Symbol};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::{c_double, c_int};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

fn c_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("test_build/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest.join("target/debug/libdriver.so")
}

fn tempfile_in_target() -> File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push(format!("capture-{}-{}.tmp", std::process::id(), n));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open tempfile");
    let _ = std::fs::remove_file(&path);
    f
}

fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe { fflush(std::ptr::null_mut()); }
    let tmp = tempfile_in_target();
    let saved_fd: c_int = unsafe { dup(1) };
    assert!(saved_fd >= 0, "dup(stdout) failed");
    let r = unsafe { dup2(tmp.as_raw_fd(), 1) };
    assert!(r >= 0, "dup2 failed");
    f();
    unsafe { fflush(std::ptr::null_mut()); }
    unsafe {
        dup2(saved_fd, 1);
        close(saved_fd);
    }
    let mut tmp = tmp;
    tmp.seek(SeekFrom::Start(0)).expect("seek tmp");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read tmp");
    out
}

type DriverFn = unsafe extern "C" fn(c_double);

fn run_driver(driver: &Symbol<DriverFn>, f: c_double) -> Vec<u8> {
    capture_stdout(|| unsafe { driver(f) })
}

fn assert_match(c: &Symbol<DriverFn>, r: &Symbol<DriverFn>, input: c_double) {
    let c_out = run_driver(c, input);
    let r_out = run_driver(r, input);
    assert_eq!(
        c_out,
        r_out,
        "mismatch for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        input,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

#[test]
fn driver_outputs_match_for_all_inputs() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c: Symbol<DriverFn> = unsafe { c_lib.get(b"driver").expect("C driver") };
    let r: Symbol<DriverFn> = unsafe { r_lib.get(b"driver").expect("Rust driver") };

    let cases: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        std::f64::consts::PI,
        std::f64::consts::E,
        std::f64::consts::LN_2,
        std::f64::consts::SQRT_2,
        1.234e-300,
        1.234e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::MIN_POSITIVE,
        f64::from_bits(1),       // smallest subnormal
        f64::from_bits(0x7ff0_0000_0000_0001), // signaling NaN
        f64::from_bits(0x7ff8_0000_0000_0001), // quiet NaN with payload
        f64::MAX,
        f64::MIN,
        0.00005,                 // %.4f rounding boundary
        -0.00005,
        0.00004999999,
        0.00005000001,
        1.0 / 3.0,
        2.0 / 3.0,
        100.0,
        -100.0,
        12345.6789,
        -12345.6789,
        9999.99995,              // rounds up to 10000.0000
        1e-308,
        1e308,
        4.9406564584124654e-324, // smallest positive denormal
        2.2250738585072009e-308, // largest denormal
        2.2250738585072014e-308, // smallest normal
    ];

    for &input in cases {
        assert_match(&c, &r, input);
    }
}

#[test]
fn driver_main_symbol_present() {
    // The Rust .so must export `main` with the exact same name as the C .so.
    let rust = unsafe { Library::new(rust_lib_path()).expect("load rust") };
    let _: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { rust.get(b"main").expect("main not exported by Rust .so") };
    let c = unsafe { Library::new(c_lib_path()).expect("load c") };
    let _: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { c.get(b"main").expect("main not exported by C .so") };
}
