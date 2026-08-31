//! Shared helpers for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading` and called
//! only through their exported C symbols, so the `#[no_mangle]` export wrappers
//! of the Rust crate are exercised exactly as an external C caller would.

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    /// `fflush(NULL)` flushes every open output stream, which covers the
    /// `stdout` `FILE` object shared by both libraries and this test binary.
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;
const STDOUT_FILENO: c_int = 1;

/// Redirecting file descriptor 1 is process-global, so only one capture may be
/// in flight at a time.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with file descriptor 1 pointed at a temporary file and returns
/// every byte that was written to it.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{:p}.out",
        std::process::id(),
        &_guard as *const _
    ));
    let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        // Flush anything already pending so it is not misattributed to `f`.
        fflush(std::ptr::null_mut());

        let tmp = open(c_path.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(tmp >= 0, "failed to open capture file {}", path.display());
        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0, "failed to dup stdout");
        assert!(dup2(tmp, STDOUT_FILENO) >= 0, "failed to redirect stdout");

        f();

        // The libraries write through the C `stdout` stream; flush before the
        // descriptor is restored so all bytes land in the capture file.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, STDOUT_FILENO) >= 0, "failed to restore stdout");
        close(saved);

        lseek(tmp, 0, 0 /* SEEK_SET */);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(tmp, buf.as_mut_ptr() as *mut c_void, buf.len());
            assert!(n >= 0, "read from capture file failed");
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(tmp);
        let _ = std::fs::remove_file(&path);
        out
    }
}

/// Absolute path of the C shared library produced by the CMake build.
pub fn c_library_path() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let path = workspace.join("c_src/build/libdriver.so");
    assert!(
        path.is_file(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        path.display()
    );
    path
}

/// Absolute path of the `cdylib` produced for this crate by the current
/// `cargo test` invocation.
///
/// `cargo test` does not build `cdylib` artifacts on its own, so the library is
/// built on demand with the same profile and feature set as the test binary.
pub fn rust_library_path() -> PathBuf {
    // The test binary lives in `<target>/<profile>/deps/`, so the parent
    // directory holds the cdylib for the exact profile under test.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let path = dir.join("libdriver.so");

    static BUILT: OnceLock<()> = OnceLock::new();
    BUILT.get_or_init(|| build_cdylib(&dir));

    assert!(
        path.is_file(),
        "Rust cdylib not found at {} even after `cargo build`.",
        path.display()
    );
    path
}

/// Builds the crate's `cdylib` for the profile whose output directory is `dir`.
fn build_cdylib(dir: &Path) {
    let profile = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug")
        .to_string();

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    if profile != "debug" {
        cmd.arg("--profile").arg(&profile);
    }

    // Reproduce the feature selection of this test binary so the cdylib under
    // test matches the configuration being exercised. The crate currently
    // declares no features; `enabled_features` stays empty in that case and the
    // default (feature-less) configuration is built.
    cmd.arg("--no-default-features");
    let features = enabled_features();
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }

    // Avoid inheriting cargo's own build state from the surrounding
    // `cargo test`, which would otherwise confuse the nested invocation.
    for var in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_MAKEFLAGS"] {
        cmd.env_remove(var);
    }

    let out = cmd.output().expect("failed to run `cargo build --lib`");
    assert!(
        out.status.success(),
        "`cargo build --lib` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Cargo features that are active in this test binary.
pub fn enabled_features() -> Vec<&'static str> {
    // The crate declares no `[features]`; this list is the single place to
    // extend if that ever changes.
    Vec::new()
}


/// Both implementations of `void driver(int)`, loaded from their shared objects.
pub struct Drivers {
    // Kept alive for as long as the function pointers are used.
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    c: libloading::Symbol<'static, unsafe extern "C" fn(c_int)>,
    rust: libloading::Symbol<'static, unsafe extern "C" fn(c_int)>,
}

impl Drivers {
    pub fn load() -> Self {
        unsafe {
            let c_lib = libloading::Library::new(c_library_path()).expect("load C .so");
            let rust_lib = libloading::Library::new(rust_library_path()).expect("load Rust .so");
            let c: libloading::Symbol<unsafe extern "C" fn(c_int)> =
                c_lib.get(b"driver\0").expect("C .so exports `driver`");
            let rust: libloading::Symbol<unsafe extern "C" fn(c_int)> =
                rust_lib.get(b"driver\0").expect("Rust .so exports `driver`");
            // Extend the symbol lifetimes; the owning libraries are stored in
            // the same struct and dropped after the symbols.
            Self {
                c: std::mem::transmute::<
                    libloading::Symbol<'_, unsafe extern "C" fn(c_int)>,
                    libloading::Symbol<'static, unsafe extern "C" fn(c_int)>,
                >(c),
                rust: std::mem::transmute::<
                    libloading::Symbol<'_, unsafe extern "C" fn(c_int)>,
                    libloading::Symbol<'static, unsafe extern "C" fn(c_int)>,
                >(rust),
                _c_lib: c_lib,
                _rust_lib: rust_lib,
            }
        }
    }

    /// Stdout bytes produced by the C `driver(x)`.
    pub fn c_output(&self, x: c_int) -> Vec<u8> {
        capture_stdout(|| unsafe { (self.c)(x) })
    }

    /// Stdout bytes produced by the Rust `driver(x)`.
    pub fn rust_output(&self, x: c_int) -> Vec<u8> {
        capture_stdout(|| unsafe { (self.rust)(x) })
    }
}
