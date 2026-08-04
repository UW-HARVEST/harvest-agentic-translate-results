// Integration test that loads both the C .so and the Rust .so via libloading
// and compares the byte-for-byte outputs of their exported functions.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Global lock to serialize stdout-capturing tests so parallel test threads
// don't fight over fd 1.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libhello.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib target produced by `cargo build` lives in target/debug.
    // CARGO_MANIFEST_DIR points to the project root.
    let candidate = project_root().join("target/debug/libhello.so");
    if candidate.exists() {
        return candidate;
    }
    project_root().join("target/release/libhello.so")
}

/// Run a closure with stdout redirected to a temp file, then return the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = STDOUT_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // Flush stdout first.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let saved_stdout: c_int = unsafe { libc::dup(1) };
    assert!(saved_stdout >= 0, "dup(stdout) failed");

    let tmp_path = std::env::temp_dir().join(format!(
        "captured_stdout_{}_{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let tmp = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create temp file");
    let tmp_fd = tmp.as_raw_fd();

    unsafe {
        libc::dup2(tmp_fd, 1);
    }

    // Run user's closure.
    f();

    // Flush libc stdout for any C-side printf.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Restore stdout and close saved.
    unsafe {
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);
    }

    // Read captured bytes.
    let mut tmp = tmp;
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).expect("read");
    drop(tmp);
    let _ = std::fs::remove_file(&tmp_path);
    buf
}

#[test]
fn helloworld_matches_between_c_and_rust() {
    let c_path = c_lib_path();
    assert!(
        c_path.exists(),
        "C shared library not found at {} - build with cmake first",
        c_path.display()
    );
    let rust_path = rust_lib_path();
    assert!(
        rust_path.exists(),
        "Rust shared library not found at {} - run `cargo build` first",
        rust_path.display()
    );

    let c_lib = unsafe { Library::new(&c_path) }.expect("load C lib");
    let rust_lib = unsafe { Library::new(&rust_path) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { c_lib.get(b"helloworld\0") }.expect("C helloworld symbol");
    let rust_fn: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { rust_lib.get(b"helloworld\0") }.expect("Rust helloworld symbol");

    // Capture C output and return value.
    let mut c_ret: c_int = 0;
    let c_out = capture_stdout(|| {
        c_ret = unsafe { c_fn() };
    });

    // Capture Rust output and return value.
    let mut r_ret: c_int = 0;
    let r_out = capture_stdout(|| {
        r_ret = unsafe { rust_fn() };
    });

    assert_eq!(c_ret, r_ret, "return values differ");
    assert_eq!(
        c_out, r_out,
        "stdout bytes differ\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );

    // Sanity: ensure expected text was produced.
    assert_eq!(c_out, b"Hello World!\n");
}

#[test]
fn rust_so_exports_helloworld() {
    let rust_path = rust_lib_path();
    let lib = unsafe { Library::new(&rust_path) }.expect("load Rust lib");
    let _: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { lib.get(b"helloworld\0") }.expect("Rust .so must export helloworld");
}
