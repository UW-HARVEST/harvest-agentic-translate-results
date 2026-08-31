//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported `driver` symbol, so the `#[no_mangle]`
//! export wrapper is part of what is under test.
//!
//! `driver` communicates only through `stdout`, so "comparing outputs" means
//! capturing file descriptor 1 around each call and comparing the raw bytes.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes every C stdio stream. The test binary and the
    /// loaded `.so`s share one glibc instance, so this drains the buffer that
    /// the C `printf` wrote into.
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(status: i32) -> !;
}

pub type DriverFn = unsafe extern "C" fn(std::ffi::c_int, std::ffi::c_int);

/// The workspace root, i.e. the directory holding `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Builds and locates the Rust `cdylib` under test.
///
/// `cargo test` does **not** refresh a `cdylib` artifact, so simply reading
/// `target/<profile>/libdriver.so` can silently test a stale library that no
/// longer corresponds to `src/lib.rs`. The library is therefore rebuilt here,
/// into a dedicated target directory so the nested `cargo` cannot contend for
/// the build lock held by the outer `cargo test`.
fn rust_library_path() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let target_dir = manifest.join("target/test-cdylib");
            let profile = if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            };

            let mut cmd = std::process::Command::new(env!("CARGO"));
            cmd.arg("build")
                .arg("--manifest-path")
                .arg(manifest.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(&target_dir)
                .arg("--lib")
                .arg("--no-default-features");
            if profile == "release" {
                cmd.arg("--release");
            }
            // Reproduce the exact feature set these tests were compiled with.
            let features = env!("DRIVER_ACTIVE_FEATURES");
            if !features.is_empty() {
                cmd.arg("--features").arg(features);
            }

            let out = cmd.output().expect("failed to spawn cargo to build the cdylib");
            assert!(
                out.status.success(),
                "failed to build the Rust cdylib under test (features: {:?})\n{}",
                features,
                String::from_utf8_lossy(&out.stderr)
            );

            let so = target_dir.join(profile).join("libdriver.so");
            assert!(
                so.is_file(),
                "cargo reported success but {} does not exist",
                so.display()
            );
            so
        })
        .clone()
}

/// The freshly built Rust `cdylib` under test.
pub fn rust_so_path() -> PathBuf {
    rust_library_path()
}

/// The C ground-truth shared library.
pub fn c_so_path() -> PathBuf {
    c_library_path()
}

/// The two implementations under comparison, kept alive for the whole test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: DriverFn,
    rust: DriverFn,
}

impl Pair {
    pub fn load() -> Pair {
        // SAFETY: both objects are plain C ABI libraries with no initialisers
        // that could misbehave. `Library::new` uses RTLD_LOCAL, so the two
        // identically named `driver` symbols do not collide.
        unsafe {
            let c_lib = Library::new(c_library_path()).expect("failed to dlopen the C .so");
            let rust_lib = Library::new(rust_library_path()).expect("failed to dlopen the Rust .so");

            let c: Symbol<DriverFn> = c_lib
                .get(b"driver\0")
                .expect("the C .so does not export `driver`");
            let rust: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("the Rust .so does not export `driver`");

            let c = *c;
            let rust = *rust;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Calls the C `driver` and returns everything it wrote to stdout.
    pub fn call_c(&self, x: i32, y: i32) -> Vec<u8> {
        let f = self.c;
        capture_stdout(|| unsafe { f(x, y) })
    }

    /// Calls the Rust `driver` and returns everything it wrote to stdout.
    pub fn call_rust(&self, x: i32, y: i32) -> Vec<u8> {
        let f = self.rust;
        capture_stdout(|| unsafe { f(x, y) })
    }

    /// Asserts that both implementations emit byte-identical stdout.
    pub fn assert_same(&self, x: i32, y: i32) {
        let c_out = self.call_c(x, y);
        let rust_out = self.call_rust(x, y);
        assert_eq!(
            c_out,
            rust_out,
            "stdout mismatch for driver({x}, {y})\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );
    }

    pub fn c_fn(&self) -> DriverFn {
        self.c
    }

    pub fn rust_fn(&self) -> DriverFn {
        self.rust
    }
}

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Guards every manipulation of file descriptor 1 *and* every `fork`.
///
/// Integration tests run as parallel threads in one process, and fd 1 is
/// process-global: without this, a forked child from one test can write into
/// another test's capture file. `fork` is covered by the same lock because a
/// child inherits whatever fd 1 currently points at.
static FD1_LOCK: Mutex<()> = Mutex::new(());

/// Redirects fd 1 to a temporary file for the duration of `f` and returns the
/// bytes written.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = FD1_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_cap_{}_{}", std::process::id(), seq));

    let mut buf = Vec::new();
    {
        let file = std::fs::File::create(&path).expect("failed to create capture file");
        // SAFETY: plain POSIX fd juggling; `saved` is restored before returning.
        unsafe {
            // Drain anything already pending so it is not attributed to `f`:
            // both the C stdio buffers and this binary's own Rust stdout, which
            // is block-buffered when cargo pipes it.
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

            f();

            // The C implementation's stdout is fully buffered when redirected to
            // a file, so it must be flushed while fd 1 still points at the file.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "failed to restore fd 1");
            close(saved);
        }
    }

    std::fs::File::open(&path)
        .expect("failed to reopen capture file")
        .read_to_end(&mut buf)
        .expect("failed to read capture file");
    let _ = std::fs::remove_file(&path);
    buf
}

/// How a forked child terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
}

/// Runs `f` in a forked child and reports how the child terminated.
///
/// Needed for the inputs C leaves undefined (`y == 0`, and `INT_MIN / -1`),
/// which trap with `SIGFPE` on x86-64 and would otherwise kill the test process.
pub fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    let _guard = FD1_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // The child's stdout is pointed at /dev/null so that anything it prints can
    // never be mistaken for another test's captured output.
    let devnull = std::fs::File::create("/dev/null").expect("failed to open /dev/null");

    // SAFETY: the child performs the call and then `_exit`s immediately without
    // returning into the test harness or flushing inherited stdio buffers.
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            dup2(devnull.as_raw_fd(), 1);
            f();
            _exit(0);
        }
        let mut status: i32 = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid() failed");
        if status & 0x7f == 0x7f {
            // stopped; not expected here
            Outcome::Signalled(-1)
        } else if status & 0x7f != 0 {
            Outcome::Signalled(status & 0x7f)
        } else {
            Outcome::Exited((status >> 8) & 0xff)
        }
    }
}
