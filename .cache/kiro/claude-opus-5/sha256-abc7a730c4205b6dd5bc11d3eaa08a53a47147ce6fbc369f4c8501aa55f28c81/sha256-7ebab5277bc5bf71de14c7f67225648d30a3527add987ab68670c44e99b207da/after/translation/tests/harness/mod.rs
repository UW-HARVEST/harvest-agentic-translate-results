//! Shared helpers: locate both shared libraries and capture what they print
//! on the process' stdout file descriptor.
//!
//! Both libraries are loaded into this single process and both call into the
//! platform's `printf`, so the capture has to happen at the file-descriptor
//! level (and `fflush(NULL)` has to run before every hand-over) rather than
//! through Rust's `std::io` machinery.

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut core::ffi::c_void, count: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const SEEK_SET: c_int = 0;

/// Directory holding this test binary's build artifacts, i.e. the profile
/// directory (`target/debug` or `target/release`) that also contains the
/// freshly built `libdriver.so` for the feature set under test.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>
    exe.parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Path to the shared library produced from `c_src/`.
pub fn c_lib_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library missing at {}; build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the shared library produced from the Rust crate.
///
/// The crate is a pure `cdylib`, so cargo does **not** build it as a dependency
/// of an integration test: a `libdriver.so` sitting in the profile directory
/// may be arbitrarily old. It is therefore rebuilt unconditionally here, once
/// per test binary, so the tests can never pass against a stale artifact.
///
/// `DRIVER_TEST_CARGO_ARGS` carries the feature selection under test (set by
/// `verify_all_features.sh`) so the library matches the configuration the test
/// itself was compiled for.
pub fn rust_lib_path() -> PathBuf {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(build_rust_lib).clone()
}

fn build_rust_lib() -> PathBuf {
    let dir = profile_dir();
    let p = dir.join("libdriver.so");
    let profile = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build");
    if profile != "debug" {
        cmd.arg("--profile").arg(&profile);
    }
    if let Ok(extra) = std::env::var("DRIVER_TEST_CARGO_ARGS") {
        for a in extra.split_whitespace() {
            cmd.arg(a);
        }
    }
    // Keep the child's progress output off fd 1 so it can never interleave
    // with a capture; diagnostics still reach stderr.
    cmd.stdout(std::process::Stdio::null());
    let status = cmd.status();
    assert!(
        matches!(&status, Ok(s) if s.success()),
        "failed to build the Rust cdylib ({status:?})"
    );
    assert!(
        p.exists(),
        "Rust shared library missing at {} after a successful build",
        p.display()
    );
    p
}

/// The two libraries under comparison, kept loaded for the lifetime of a test.
pub struct Libs {
    pub c: libloading::Library,
    pub rust: libloading::Library,
}

impl Libs {
    pub fn load() -> Self {
        // SAFETY: both paths point at shared objects built from the sources in
        // this repository; loading them runs their (empty) initialisers.
        unsafe {
            Self {
                c: libloading::Library::new(c_lib_path()).expect("load C libdriver.so"),
                rust: libloading::Library::new(rust_lib_path()).expect("load Rust libdriver.so"),
            }
        }
    }
}

/// Serialises stdout redirection: file descriptor 1 is process-wide, so two
/// tests capturing at the same time would steal each other's output.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Redirects file descriptor 1 into a temporary file, runs `f`, and returns
/// every byte that was written.
///
/// `fflush(NULL)` is issued on both sides of the swap so that stdio buffers
/// belonging to either library end up in the right capture.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let cpath = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

    unsafe {
        // Two independent layers buffer writes to fd 1: the C stdio buffers
        // used by both libraries, and the `LineWriter` behind Rust's
        // `std::io::stdout` that the libtest harness prints progress through.
        // Both must be drained, otherwise their leftovers land in this
        // capture. (Notably `test <name> ... ` has no trailing newline, so the
        // line writer holds on to it.)
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());

        let tmp = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(tmp >= 0, "open temp capture file");
        let saved = dup(1);
        assert!(saved >= 0, "dup stdout");
        assert!(dup2(tmp, 1) >= 0, "dup2 temp -> stdout");

        f();

        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());
        assert!(dup2(saved, 1) >= 0, "restore stdout");
        close(saved);

        lseek(tmp, 0, SEEK_SET);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(tmp, buf.as_mut_ptr().cast(), buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(tmp);
        unlink(cpath.as_ptr());
        out
    }
}
