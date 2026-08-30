//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading`, so the
//! Rust side is exercised exactly like an external C caller would: only the
//! `#[no_mangle]` exported symbols are ever touched.
//!
//! The functions under test communicate exclusively via `printf` on stdout, so
//! the harness captures file descriptor 1 around each call. That is a
//! process-global operation, hence the mutex.

// This module is compiled into every integration-test binary, and not every
// binary uses every helper (the symbol-parity test needs no stdout capture).
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open stdio stream, including the one the
    /// loaded libraries write to (they share this process' libc).
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;

/// Serialises the fd-1 redirection performed by [`capture_stdout`].
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locates `libdriver.so` produced by the Rust build.
///
/// The test executable lives in `target/<profile>/deps/`, so the cdylib sits
/// one directory up. This keeps the lookup correct for both `debug` and
/// `release` profiles without hard-coding either.
fn rust_library_path() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    let deps_dir = exe.parent().expect("deps dir");
    let candidates = [
        deps_dir.join("libdriver.so"),
        deps_dir
            .parent()
            .expect("profile dir")
            .join("libdriver.so"),
    ];
    for candidate in &candidates {
        if candidate.is_file() {
            return candidate.clone();
        }
    }
    panic!(
        "could not find the Rust cdylib; looked in {candidates:?}. \
         Run `cargo build` first."
    );
}

/// Locates the C `libdriver.so`, building it with CMake if it is not there yet.
fn c_library_path() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build_dir = c_src.join("build");
    let so = build_dir.join("libdriver.so");
    if so.is_file() {
        return so;
    }

    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let configure = std::process::Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build_dir)
        .status()
        .expect("run cmake configure");
    assert!(configure.success(), "cmake configure failed");
    let build = std::process::Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .status()
        .expect("run cmake build");
    assert!(build.success(), "cmake build failed");

    assert!(so.is_file(), "cmake did not produce {}", so.display());
    so
}

/// The two libraries under comparison, loaded once for the whole test binary.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        // SAFETY: both objects are plain C ABI libraries built from this repo.
        unsafe {
            let c = Library::new(c_library_path()).expect("load C libdriver.so");
            let rust = Library::new(rust_library_path()).expect("load Rust libdriver.so");
            Libs { c, rust }
        }
    })
}

/// Signature shared by `driver` and `printHexCharLine`.
pub type CharFn = unsafe extern "C" fn(c_char);

/// Resolves `name` from both libraries, asserting the symbol exists in each.
pub fn char_fns<'a>(name: &str) -> (Symbol<'a, CharFn>, Symbol<'a, CharFn>) {
    let l = libs();
    let bytes = name.as_bytes();
    // SAFETY: the symbols are `void (char)` in the C header and are declared
    // with the identical signature in the Rust translation.
    unsafe {
        let c = l
            .c
            .get::<CharFn>(&[bytes, b"\0"].concat())
            .unwrap_or_else(|e| panic!("C library is missing `{name}`: {e}"));
        let rust = l
            .rust
            .get::<CharFn>(&[bytes, b"\0"].concat())
            .unwrap_or_else(|e| panic!("Rust library is missing `{name}`: {e}"));
        (c, rust)
    }
}

/// Runs `body` with stdout redirected to a temporary file and returns every
/// byte written to file descriptor 1, including output produced from inside the
/// loaded shared objects.
pub fn capture_stdout<F: FnOnce()>(body: F) -> Vec<u8> {
    use std::io::Write;

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("driver-capture-{}-{seq}.out", std::process::id()));

    // SAFETY: raw fd juggling; every descriptor obtained below is restored or
    // closed before returning.
    unsafe {
        // Drain anything already buffered - both Rust's stdout handle and the
        // C stdio stream - so it is not attributed to `body`.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let file = std::fs::File::create(&tmp).expect("create capture file");
        let file_fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };

        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0, "dup(stdout) failed");
        assert!(dup2(file_fd, STDOUT_FILENO) >= 0, "dup2 onto stdout failed");

        body();

        // Push the C stdio buffer into the file before restoring stdout,
        // otherwise fully-buffered output would land on the real stdout later.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FILENO) >= 0, "restore stdout failed");
        close(saved);
        drop(file);
    }

    let out = std::fs::read(&tmp).expect("read capture file");
    let _ = std::fs::remove_file(&tmp);
    out
}

/// Calls the same exported symbol in both libraries with `arg` and asserts the
/// captured stdout bytes are identical.
pub fn assert_char_fn_matches(name: &str, arg: c_char) {
    let (c_fn, rust_fn) = char_fns(name);

    // SAFETY: `void (char)`, no pointers involved.
    let c_out = capture_stdout(|| unsafe { c_fn(arg) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(arg) });

    assert_eq!(
        c_out,
        rust_out,
        "{name}({arg}) [0x{:02x}] mismatch:\n  C   : {:?} ({:x?})\n  Rust: {:?} ({:x?})",
        arg as u8,
        String::from_utf8_lossy(&c_out),
        c_out,
        String::from_utf8_lossy(&rust_out),
        rust_out,
    );
}
