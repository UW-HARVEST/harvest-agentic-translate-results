//! Shared harness: loads the C and the Rust shared libraries through
//! `libloading` and captures whatever they write to `stdout` so the two can be
//! compared byte for byte.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn repo_root() -> PathBuf {
    // tests/ live in translation/, the C tree is its sibling.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    repo_root().join("c_src/build/libdriver.so")
}

/// The Rust `cdylib` for the profile currently under test.
///
/// `cargo test` puts integration test binaries in `target/<profile>/deps`, so
/// the sibling `libdriver.so` is the artifact built from the same feature set.
fn rust_lib_path() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().is_some_and(|n| n == "deps") {
        dir.pop();
    }
    let candidate = dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    // Fall back to whichever profile directory actually holds the artifact.
    for profile in ["debug", "release"] {
        let p = repo_root().join("translation/target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    candidate
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

/// Both libraries, loaded once per test binary.
///
/// They are leaked for the lifetime of the process: unloading a `cdylib` that
/// registered atexit handlers mid-test would be needlessly fragile.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_lib_path();
        let rust_path = rust_lib_path();
        assert!(
            c_path.exists(),
            "C shared library missing at {}; build it with cmake first",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library missing at {}; run cargo build first",
            rust_path.display()
        );
        unsafe {
            Libs {
                c: Library::new(&c_path).expect("dlopen C library"),
                rust: Library::new(&rust_path).expect("dlopen Rust library"),
            }
        }
    })
}

pub type DriverFn = unsafe extern "C" fn(c_int);
pub type PrintLineFn = unsafe extern "C" fn(*const c_char);

pub fn driver_fns() -> (Symbol<'static, DriverFn>, Symbol<'static, DriverFn>) {
    let l = libs();
    unsafe {
        (
            l.c.get(b"driver\0").expect("C driver symbol"),
            l.rust.get(b"driver\0").expect("Rust driver symbol"),
        )
    }
}

pub fn print_line_fns() -> (Symbol<'static, PrintLineFn>, Symbol<'static, PrintLineFn>) {
    let l = libs();
    unsafe {
        (
            l.c.get(b"printLine\0").expect("C printLine symbol"),
            l.rust.get(b"printLine\0").expect("Rust printLine symbol"),
        )
    }
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// the raw bytes it produced.
///
/// `fflush(NULL)` is used on both sides of the redirection so that anything
/// still sitting in the shared libc `stdout` buffer is attributed to the right
/// side of the comparison.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;

    // fd 1 is process-global, so only one capture may be in flight at a time
    // even though the test harness runs tests on several threads.
    static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!(
        "driver_capture_{}_{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create capture file");

    unsafe {
        // Flush anything buffered before we steal fd 1.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        f();

        // The redirected fd 1 is a regular file, hence fully buffered; force
        // the bytes out before restoring the original descriptor.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore stdout failed");
        close(saved);
    }

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("rewind capture file");
    file.read_to_end(&mut out).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&tmp_path);
    out
}

/// Formats a byte string for assertion messages without hiding non-UTF-8 bytes.
pub fn show(bytes: &[u8]) -> String {
    format!("{} bytes: {:?}", bytes.len(), String::from_utf8_lossy(bytes))
}
