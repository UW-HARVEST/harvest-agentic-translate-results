//! Shared harness: loads the C and Rust shared libraries with `libloading` and
//! captures everything each one writes to file descriptor 1 so the two byte
//! streams can be compared.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::ffi::CString;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;

use libloading::Library;
use libloading::Symbol;

// Raw libc entry points needed to redirect and restore stdout. Declared here so
// the test harness needs no dependency beyond `libloading`.
unsafe extern "C" {
    unsafe fn dup(oldfd: c_int) -> c_int;
    unsafe fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    unsafe fn close(fd: c_int) -> c_int;
    unsafe fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    unsafe fn fflush(stream: *mut c_void) -> c_int;
}

const O_WRONLY: c_int = 0o1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const STDOUT_FD: c_int = 1;

/// stdout redirection is process-wide, so captures must not overlap.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// The workspace root (parent of the `translation` crate directory).
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory has a parent")
        .to_path_buf()
}

/// Path to the C shared library produced by the CMake build.
pub fn c_library_path() -> PathBuf {
    let path = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        path.exists(),
        "C shared library missing at {}; build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        path.display()
    );
    path
}

/// Path to the Rust `cdylib` for the profile the tests were built with.
///
/// `cargo test` does not build a `cdylib`-only lib target (nothing links it into
/// the integration test binaries), so the artifact is produced on demand here.
pub fn rust_library_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        // The test executable lives in `<target>/<profile>/deps/`, so the cdylib
        // built alongside it is two levels up.
        let exe = std::env::current_exe().expect("test executable path");
        let profile_dir = exe
            .parent()
            .and_then(Path::parent)
            .expect("target profile directory")
            .to_path_buf();
        let path = profile_dir.join("libdriver.so");

        if !path.exists() {
            let profile = profile_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("debug")
                .to_string();
            build_cdylib(&profile);
        }

        assert!(
            path.exists(),
            "Rust shared library missing at {} even after `cargo build`",
            path.display()
        );
        path
    })
    .clone()
}

/// Invokes `cargo build` for the crate under test so the `cdylib` exists.
fn build_cdylib(profile: &str) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build");
    if profile == "release" {
        cmd.arg("--release");
    } else if profile != "debug" {
        cmd.args(["--profile", profile]);
    }
    cmd.arg("--no-default-features");
    let features = enabled_features().join(",");
    if !features.is_empty() {
        cmd.args(["--features", &features]);
    }

    let status = cmd.status().expect("run cargo build for the cdylib");
    assert!(status.success(), "cargo build of the cdylib failed");
}

/// Cargo features active in this test binary, recorded at compile time.
///
/// The crate currently declares no `[features]`, so this is always empty; the
/// hook keeps the on-demand build correct if features are added later.
pub fn enabled_features() -> Vec<String> {
    Vec::new()
}

/// Both implementations, loaded as external shared objects.
pub struct Pair {
    pub c: Library,
    pub rust: Library,
}

impl Pair {
    pub fn load() -> Self {
        // SAFETY: both objects are plain C-ABI libraries with no initialisers
        // that require special handling.
        unsafe {
            Self {
                c: Library::new(c_library_path()).expect("load C library"),
                rust: Library::new(rust_library_path()).expect("load Rust library"),
            }
        }
    }
}

/// A `void fn(void)` exported by a shared object.
pub type VoidFn = unsafe extern "C" fn();
/// A `void fn(const char *)` exported by a shared object.
pub type StrFn = unsafe extern "C" fn(*const c_char);

pub fn void_fn<'lib>(lib: &'lib Library, name: &str) -> Symbol<'lib, VoidFn> {
    unsafe { lib.get(CString::new(name).unwrap().as_bytes_with_nul()) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not exported: {e}"))
}

pub fn str_fn<'lib>(lib: &'lib Library, name: &str) -> Symbol<'lib, StrFn> {
    unsafe { lib.get(CString::new(name).unwrap().as_bytes_with_nul()) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not exported: {e}"))
}

/// Runs `body` with file descriptor 1 pointed at a temporary file and returns
/// the raw bytes written. C stdio buffers are flushed before and after so no
/// output leaks across captures.
pub fn capture_stdout<F: FnOnce()>(body: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver-capture-{}-{:p}.out",
        std::process::id(),
        &path
    ));
    let c_path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

    // SAFETY: raw fd juggling; every descriptor obtained below is closed or
    // restored before returning.
    let saved = unsafe {
        // Flush anything already pending on the real stdout.
        fflush(std::ptr::null_mut());
        let file = open(c_path.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int);
        assert!(file >= 0, "cannot open capture file {}", path.display());
        let saved = dup(STDOUT_FD);
        assert!(saved >= 0, "cannot duplicate stdout");
        assert!(dup2(file, STDOUT_FD) >= 0, "cannot redirect stdout");
        close(file);
        saved
    };

    body();

    unsafe {
        // Flush the library's buffered output into the capture file.
        fflush(std::ptr::null_mut());
        dup2(saved, STDOUT_FD);
        close(saved);
    }

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Renders captured bytes for assertion messages without assuming UTF-8.
pub fn render(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts the two captures are byte-identical.
pub fn assert_same(label: &str, c_out: &[u8], rust_out: &[u8]) {
    assert_eq!(
        c_out,
        rust_out,
        "output mismatch for {label}\n  C   ({} bytes): \"{}\"\n  Rust({} bytes): \"{}\"",
        c_out.len(),
        render(c_out),
        rust_out.len(),
        render(rust_out)
    );
}
