//! Shared harness: loads the C and Rust shared libraries through `libloading`
//! and captures everything they write to file descriptor 1 (stdout).
//!
//! The Rust side is *never* called directly — only through symbols resolved
//! from the built `cdylib`, so the `#[no_mangle]` wrappers are exercised too.

// Each integration-test binary includes this module and uses a different subset.
#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

/// Repository root (the directory holding `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C shared library produced by CMake.
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = repo_root();
    for cand in [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/libdriver.dylib"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
}

/// Path to the Rust `cdylib`. Prefers an explicit override, then release,
/// then the debug artifact that `cargo test` itself builds.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    for cand in [
        manifest.join("target/release/libdriver.so"),
        manifest.join("target/debug/libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` in translation/");
}

/// `void driver(int)` as seen through the FFI boundary.
pub type DriverFn = unsafe extern "C" fn(c_int);

pub struct Libs {
    // Keep the libraries alive for the whole process; symbols borrow from them.
    _c: &'static libloading::Library,
    _rust: &'static libloading::Library,
    pub c_driver: DriverFn,
    pub rust_driver: DriverFn,
}

fn leak_load(path: &Path) -> &'static libloading::Library {
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    Box::leak(Box::new(lib))
}

/// Loads both libraries once and resolves `driver` from each handle.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c = leak_load(&c_lib_path());
        let rust = leak_load(&rust_lib_path());
        let c_driver: DriverFn = unsafe {
            *c.get::<DriverFn>(b"driver\0")
                .expect("C library does not export `driver`")
        };
        let rust_driver: DriverFn = unsafe {
            *rust
                .get::<DriverFn>(b"driver\0")
                .expect("Rust library does not export `driver`")
        };
        Libs {
            _c: c,
            _rust: rust,
            c_driver,
            rust_driver,
        }
    })
}

/// Serialises the fd-1 redirection performed by [`capture_stdout`].
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Runs `f` in a forked child whose file descriptor 1 points at a temporary
/// file, and returns the raw bytes it wrote.
///
/// A child process is used rather than an in-process `dup2` swap because the
/// test harness itself writes progress lines to fd 1 from other threads; those
/// writes would otherwise be captured and compared as if they came from the
/// library. `fflush(NULL)` before the fork ensures no buffered libc output is
/// inherited, and the child uses `_exit` so it never flushes buffers a second
/// time in the parent's name.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "driver_capture_{}_{}.out",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));

    let bytes = {
        let file = std::fs::File::create(&tmp).expect("create temp capture file");
        let fd = file.as_raw_fd();

        // Flush everything the parent has buffered so the child inherits nothing.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let _ = std::io::Write::flush(&mut std::io::stderr());
        unsafe { fflush(std::ptr::null_mut()) };

        let pid = unsafe { fork() };
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // Child: point stdout at the capture file, run the call, flush, leave.
            unsafe {
                if dup2(fd, 1) < 0 {
                    _exit(101);
                }
                f();
                fflush(std::ptr::null_mut());
                _exit(0);
            }
        }

        let mut status: c_int = 0;
        let waited = unsafe { waitpid(pid, &mut status, 0) };
        assert_eq!(waited, pid, "waitpid() failed");
        // WIFEXITED / WEXITSTATUS / WTERMSIG, spelled out to avoid a libc dep.
        let exited_normally = (status & 0x7f) == 0;
        let exit_code = (status >> 8) & 0xff;
        assert!(
            exited_normally && exit_code == 0,
            "captured call terminated abnormally (raw wait status {status:#x}, signal {})",
            status & 0x7f
        );

        drop(file);
        std::fs::read(&tmp).expect("read temp capture file")
    };

    let _ = std::fs::remove_file(&tmp);
    bytes
}

/// Calls `driver(x)` in both libraries and asserts byte-identical stdout.
pub fn assert_driver_matches(x: c_int) {
    let l = libs();
    let c_out = capture_stdout(|| unsafe { (l.c_driver)(x) });
    let rust_out = capture_stdout(|| unsafe { (l.rust_driver)(x) });

    if c_out != rust_out {
        panic!(
            "driver({x}) mismatch\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
            c_out.len(),
            preview(&c_out),
            rust_out.len(),
            preview(&rust_out),
        );
    }
}

fn preview(bytes: &[u8]) -> String {
    const MAX: usize = 400;
    let shown = &bytes[..bytes.len().min(MAX)];
    let mut s = String::from_utf8_lossy(shown).escape_debug().to_string();
    if bytes.len() > MAX {
        s.push_str("...<truncated>");
    }
    s
}
