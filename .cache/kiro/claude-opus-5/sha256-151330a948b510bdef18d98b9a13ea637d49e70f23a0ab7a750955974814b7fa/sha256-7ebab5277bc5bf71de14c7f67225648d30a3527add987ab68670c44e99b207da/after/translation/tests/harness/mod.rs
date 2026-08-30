//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported (`#[no_mangle]` / C `extern`) symbols, so
//! the export wrappers themselves are part of what is under test.
//!
//! Every function in `driver.c` communicates exclusively through `printf`, so
//! "output" here means *the bytes written to file descriptor 1*. The capture
//! helper therefore redirects fd 1 (not Rust's `std::io::stdout`, which is a
//! separate buffer) around each call and flushes libc's stream buffers on both
//! sides of the redirection.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, which is what we need:
    /// the `printf` call lives inside the dlopened library but shares this
    /// process's libc `stdout`.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Which implementation a value came from, used for assertion messages.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

/// A loaded `driver` shared object plus the four symbols it must export.
pub struct Driver {
    pub which: Impl,
    pub path: PathBuf,
    // `lib` must outlive the symbols; it is dropped last (declared last).
    print_line: unsafe extern "C" fn(*const c_char),
    bad: unsafe extern "C" fn(),
    good: unsafe extern "C" fn(),
    driver: unsafe extern "C" fn(c_int),
    _lib: Library,
}

impl Driver {
    fn load(which: Impl, path: &Path) -> Driver {
        assert!(
            path.exists(),
            "shared object for {which:?} not found at {}",
            path.display()
        );
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

            // Resolving each symbol by its exact C name is itself an assertion
            // that the Rust cdylib exports the same ABI-visible names.
            let print_line: Symbol<unsafe extern "C" fn(*const c_char)> = lib
                .get(b"printLine\0")
                .unwrap_or_else(|e| panic!("{which:?}: missing symbol `printLine`: {e}"));
            let bad: Symbol<unsafe extern "C" fn()> = lib
                .get(b"bad\0")
                .unwrap_or_else(|e| panic!("{which:?}: missing symbol `bad`: {e}"));
            let good: Symbol<unsafe extern "C" fn()> = lib
                .get(b"good\0")
                .unwrap_or_else(|e| panic!("{which:?}: missing symbol `good`: {e}"));
            let driver: Symbol<unsafe extern "C" fn(c_int)> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{which:?}: missing symbol `driver`: {e}"));

            Driver {
                which,
                path: path.to_path_buf(),
                print_line: *print_line.into_raw(),
                bad: *bad.into_raw(),
                good: *good.into_raw(),
                driver: *driver.into_raw(),
                _lib: lib,
            }
        }
    }

    pub fn print_line(&self, line: *const c_char) {
        unsafe { (self.print_line)(line) }
    }

    pub fn bad(&self) {
        unsafe { (self.bad)() }
    }

    pub fn good(&self) {
        unsafe { (self.good)() }
    }

    pub fn driver(&self, use_good: c_int) {
        unsafe { (self.driver)(use_good) }
    }
}

/// Workspace root (the directory holding both `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "C shared library not built; run:\n  cd {} && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                root.join("c_src").display()
            )
        })
}

fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // The test executable lives in `target/<profile>/deps/`, so the cdylib
    // produced by the same `cargo test` invocation is one directory up. This
    // keeps the lookup correct for every profile and feature combination.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let mut candidates = vec![deps.join("libdriver.so")];
    if let Some(profile_dir) = deps.parent() {
        candidates.push(profile_dir.join("libdriver.so"));
    }
    let root = workspace_root();
    candidates.push(root.join("translation/target/debug/libdriver.so"));
    candidates.push(root.join("translation/target/release/libdriver.so"));
    candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("Rust cdylib not found; run `cargo build` in translation/"))
}

/// Loads the C implementation.
pub fn load_c() -> Driver {
    Driver::load(Impl::C, &c_so_path())
}

/// Loads the Rust implementation.
pub fn load_rust() -> Driver {
    Driver::load(Impl::Rust, &rust_so_path())
}

/// Loads both implementations.
pub fn load_both() -> (Driver, Driver) {
    (load_c(), load_rust())
}

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with file descriptor 1 pointed at a scratch file and returns every
/// byte that was written to it.
///
/// Safety/ordering notes:
/// * Rust's own `stdout` buffer is flushed first so that harness output written
///   before the capture cannot land in the captured bytes.
/// * `fflush(NULL)` runs before the redirection (drain anything pending) and
///   after `f` (force the library's `printf` output out of libc's buffer, which
///   is *fully* buffered while fd 1 is a regular file).
pub fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    // fd 1 is process-global: two captures must never overlap, even though the
    // test harness runs `#[test]` functions on separate threads.
    static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let id = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{}-{}.bin",
        std::process::id(),
        id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let bytes = {
        let file = std::fs::File::create(&path).expect("create capture file");
        let file_fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };

        unsafe {
            assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-capture fflush failed");
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file_fd, 1) >= 0, "dup2 onto fd 1 failed");

            f();

            assert_eq!(fflush(std::ptr::null_mut()), 0, "post-capture fflush failed");
            assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
            close(saved);
        }
        drop(file);
        std::fs::read(&path).expect("read capture file")
    };
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Human-readable rendering for assertion messages.
pub fn show(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() + 2);
    s.push('"');
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(b as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    s.push('"');
    s
}

/// Asserts the two captures are byte-identical.
pub fn assert_same(label: &str, c_out: &[u8], rust_out: &[u8]) {
    assert_eq!(
        c_out,
        rust_out,
        "\noutput mismatch for {label}\n  C   : {} ({} bytes)\n  Rust: {} ({} bytes)\n",
        show(c_out),
        c_out.len(),
        show(rust_out),
        rust_out.len(),
    );
}
