// Integration test that loads the C and Rust shared libraries via libloading,
// invokes the exported `driver(float)` function in each, captures stdout, and
// asserts the bytes match exactly.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::path::PathBuf;
use std::sync::Mutex;

// Serialise tests because they all redirect fd 1.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    project_root().join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    project_root().join("target/release/libdriver.so")
}

/// Run `body` while stdout (fd 1) is redirected to a temporary file.
/// Returns the bytes captured from fd 1.
fn capture_stdout<F: FnOnce()>(body: F) -> Vec<u8> {
    // Make sure libc/Rust stdout buffers are flushed before we swap the fd.
    let _ = std::io::stdout().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Save original fd 1.
    let saved_fd: RawFd = unsafe { libc::dup(1) };
    assert!(saved_fd >= 0, "dup(1) failed");

    // Create a temporary file and dup2 it onto fd 1.
    let tmp_path = std::env::temp_dir().join(format!(
        "driver_so_capture_{}_{}.bin",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut tmp = File::create(&tmp_path).expect("create tmp file");
    let tmp_fd = tmp.as_raw_fd();
    let dup_res = unsafe { libc::dup2(tmp_fd, 1) };
    assert!(dup_res >= 0, "dup2 failed");

    body();

    // Flush libc stdio before restoring fd.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    let _ = std::io::stdout().flush();

    // Restore stdout.
    let r = unsafe { libc::dup2(saved_fd, 1) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe {
        libc::close(saved_fd);
    }

    // Re-open the file to read what was written.
    drop(tmp);
    let mut buf = Vec::new();
    let mut f = File::open(&tmp_path).expect("reopen tmp file");
    f.read_to_end(&mut buf).expect("read tmp file");
    let _ = std::fs::remove_file(&tmp_path);
    buf
}

/// Call `driver(x)` from a shared library.
unsafe fn call_driver(lib: &Library, x: f32) -> Vec<u8> {
    let func: Symbol<unsafe extern "C" fn(f32)> =
        lib.get(b"driver\0").expect("driver symbol");
    capture_stdout(|| {
        func(x);
    })
}

fn run_one(x: f32) {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };
    let c_out = unsafe { call_driver(&c_lib, x) };
    let r_out = unsafe { call_driver(&r_lib, x) };
    assert_eq!(
        c_out, r_out,
        "driver({}) outputs differ:\n  C    = {:?}\n  Rust = {:?}",
        x, c_out, r_out
    );
}

#[test]
fn driver_zero() {
    run_one(0.0_f32);
}

#[test]
fn driver_negative_zero() {
    run_one(-0.0_f32);
}

#[test]
fn driver_one() {
    run_one(1.0_f32);
}

#[test]
fn driver_neg_one() {
    run_one(-1.0_f32);
}

#[test]
fn driver_pi() {
    run_one(std::f32::consts::PI);
}

#[test]
fn driver_small() {
    run_one(1.5e-30_f32);
}

#[test]
fn driver_large() {
    run_one(3.4e+30_f32);
}

#[test]
fn driver_subnormal() {
    run_one(f32::from_bits(1));
}

#[test]
fn driver_inf() {
    run_one(f32::INFINITY);
}

#[test]
fn driver_neg_inf() {
    run_one(f32::NEG_INFINITY);
}

#[test]
fn driver_nan() {
    run_one(f32::NAN);
}

#[test]
fn driver_max() {
    run_one(f32::MAX);
}

#[test]
fn driver_min_positive() {
    run_one(f32::MIN_POSITIVE);
}

#[test]
fn driver_random_bit_patterns() {
    // A handful of arbitrary bit patterns reinterpreted as f32.
    let patterns = [
        0x12345678u32,
        0xdeadbeef,
        0x80000001,
        0x7f7fffff,
        0x00000001,
        0xff800000,
        0x7fc00000,
        0x40490fdb,
        0xbf800000,
        0xc0000000,
    ];
    for p in patterns.iter() {
        run_one(f32::from_bits(*p));
    }
}

/// Sanity-check the symbol surface: every symbol exported by the C .so must
/// also be exported by the Rust .so. We compare `driver` and `main` directly.
#[test]
fn so_exports_required_symbols() {
    let c = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let r = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };

    let _: Symbol<unsafe extern "C" fn(f32)> = unsafe { c.get(b"driver\0") }.expect("C driver");
    let _: Symbol<unsafe extern "C" fn(f32)> =
        unsafe { r.get(b"driver\0") }.expect("Rust driver");

    let _: Symbol<unsafe extern "C" fn() -> c_int> = unsafe { c.get(b"main\0") }.expect("C main");
    let _: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { r.get(b"main\0") }.expect("Rust main");
}

// Avoid unused imports warning if anything is trimmed.
#[allow(dead_code)]
fn _unused_imports() {
    let _ = SeekFrom::Start(0);
    let _ = std::io::stdout();
    let _ = File::create("/dev/null").map(|f| f.into_raw_fd());
    unsafe {
        let _ = File::from_raw_fd(-1);
    }
}
