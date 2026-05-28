use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

type DriverFn = unsafe extern "C" fn(c_int);

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // Use the freshly built shared object from the cargo target dir.
    // CARGO_MANIFEST_DIR/target/debug/libdriver.so should exist after `cargo build`.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libdriver.so");
    p
}

/// Run a callable while redirecting stdout (file descriptor 1) into a
/// temporary file. Returns the bytes written to stdout during the call.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Make sure any pending Rust stdout is flushed before redirecting.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    // Also flush the C runtime stdout.
    unsafe {
        extern "C" {
            fn fflush(stream: *mut libc_stub::FILE) -> c_int;
        }
        // Use null to flush all open output streams.
        fflush(std::ptr::null_mut());
    }

    let tmp_path = std::env::temp_dir().join(format!(
        "driver_capture_{}.txt",
        std::process::id()
    ));
    // Open (truncating) a fresh temp file.
    let tmp = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create capture file");
    let tmp_fd = tmp.as_raw_fd();

    let saved_fd = unsafe { libc_stub::dup(1) };
    assert!(saved_fd >= 0);
    let dup_res = unsafe { libc_stub::dup2(tmp_fd, 1) };
    assert!(dup_res >= 0);

    f();

    // Flush before restoring fd.
    let _ = std::io::stdout().flush();
    unsafe {
        extern "C" {
            fn fflush(stream: *mut libc_stub::FILE) -> c_int;
        }
        fflush(std::ptr::null_mut());
    }

    let restore_res = unsafe { libc_stub::dup2(saved_fd, 1) };
    assert!(restore_res >= 0);
    unsafe {
        libc_stub::close(saved_fd);
    }

    // Read the captured bytes back.
    let mut tmp = tmp;
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).expect("read capture");
    drop(tmp);
    let _ = fs::remove_file(&tmp_path);
    buf
}

mod libc_stub {
    use std::ffi::c_int;
    pub enum FILE {}
    extern "C" {
        pub fn dup(fd: c_int) -> c_int;
        pub fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        pub fn close(fd: c_int) -> c_int;
    }
}

fn run_driver(lib_path: &PathBuf, x: c_int) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let lib = unsafe { Library::new(lib_path) }.expect("load library");
    let func: Symbol<DriverFn> = unsafe { lib.get(b"driver") }.expect("driver symbol");
    let out = capture_stdout(|| unsafe { func(x) });
    out
}

fn assert_match(x: c_int) {
    let c_out = run_driver(&c_so_path(), x);
    let r_out = run_driver(&rust_so_path(), x);
    assert_eq!(
        c_out, r_out,
        "Mismatch for x={}:\n  C   = {:?}\n  Rust= {:?}",
        x,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

#[test]
fn driver_zero() {
    assert_match(0);
}

#[test]
fn driver_one() {
    assert_match(1);
}

#[test]
fn driver_negative() {
    assert_match(-1);
}

#[test]
fn driver_large_positive() {
    assert_match(2_000_000_000);
}

#[test]
fn driver_large_negative() {
    assert_match(-2_000_000_000);
}

#[test]
fn driver_max() {
    assert_match(c_int::MAX);
}

#[test]
fn driver_min() {
    assert_match(c_int::MIN);
}

#[test]
fn driver_arbitrary() {
    assert_match(12345);
    assert_match(-9876);
    assert_match(0xCAFE);
    assert_match(0x55_55_55_55);
}
