//! Shared helpers for the C-vs-Rust FFI differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! exercised only through their exported C symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.
//!
//! NOTE ON PROCESS STATE: `static_sum` keeps a process-wide running total
//! (`static int sum` in C, a `static AtomicI32` in Rust). The two libraries
//! therefore each own an independent accumulator that can never be reset. All
//! comparisons are done in *lockstep*: every call made against the C library is
//! immediately mirrored against the Rust library with the same argument, so the
//! two accumulators stay in step and any divergence shows up right away.
//! Because of that, each test file must contain exactly ONE `#[test]`
//! function - cargo runs tests inside a file concurrently on multiple threads,
//! which would interleave calls non-deterministically.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Absolute path of the crate directory (`translation/`).
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Absolute path of the working directory that holds `c_src/` and
/// `translation/`.
fn project_root() -> PathBuf {
    crate_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build (if necessary) and return the path to the C shared library.
pub fn c_lib_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = project_root().join("c_src");
        let build = c_src.join("build");
        let so = build.join("libStaticLoop.so");
        if !so.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let status = Command::new("cmake")
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&build)
                .status()
                .expect("run cmake configure");
            assert!(status.success(), "cmake configure failed");
            let status = Command::new("cmake")
                .arg("--build")
                .arg(".")
                .current_dir(&build)
                .status()
                .expect("run cmake build");
            assert!(status.success(), "cmake build failed");
        }
        assert!(so.exists(), "missing C shared library at {}", so.display());
        so
    })
    .as_path()
}

/// Build (if necessary) and return the path to the Rust `cdylib`.
///
/// `cargo test` does not build `cdylib` artifacts, so the library is built here
/// into a dedicated target directory. Using a separate `--target-dir` keeps this
/// nested cargo invocation from contending on the outer build's package lock.
pub fn rust_lib_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let target_dir = crate_dir().join("target").join("ffi-artifacts");
        let so = target_dir.join("release").join("libStaticLoop.so");

        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.arg("build")
            .arg("--release")
            .arg("--target-dir")
            .arg(&target_dir)
            .current_dir(crate_dir());
        // Inherited cargo/rustup env from the outer invocation can confuse the
        // nested build; drop the variables that matter.
        for var in [
            "RUSTC",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_TARGET_DIR",
            "CARGO_MAKEFLAGS",
            "RUSTFLAGS",
        ] {
            cmd.env_remove(var);
        }
        let status = cmd.status().expect("run cargo build for the cdylib");
        assert!(status.success(), "cargo build --release of the cdylib failed");

        assert!(
            so.exists(),
            "missing Rust shared library at {}",
            so.display()
        );
        so
    })
    .as_path()
}

/// A loaded pair of implementations, addressed only through exported symbols.
pub struct Pair {
    pub c: libloading::Library,
    pub rs: libloading::Library,
}

impl Pair {
    pub fn load() -> Pair {
        // Resolve both paths (and therefore run both builds) before loading.
        let c_path = c_lib_path().to_path_buf();
        let rs_path = rust_lib_path().to_path_buf();
        unsafe {
            Pair {
                c: libloading::Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
                rs: libloading::Library::new(&rs_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display())),
            }
        }
    }

    /// `int static_sum(int)` from each library.
    pub fn static_sum_fns(
        &self,
    ) -> (
        libloading::Symbol<'_, unsafe extern "C" fn(i32) -> i32>,
        libloading::Symbol<'_, unsafe extern "C" fn(i32) -> i32>,
    ) {
        unsafe {
            (
                self.c.get(b"static_sum\0").expect("C static_sum"),
                self.rs.get(b"static_sum\0").expect("Rust static_sum"),
            )
        }
    }

    /// `void driver(int)` from each library.
    pub fn driver_fns(
        &self,
    ) -> (
        libloading::Symbol<'_, unsafe extern "C" fn(i32)>,
        libloading::Symbol<'_, unsafe extern "C" fn(i32)>,
    ) {
        unsafe {
            (
                self.c.get(b"driver\0").expect("C driver"),
                self.rs.get(b"driver\0").expect("Rust driver"),
            )
        }
    }
}

/// Run `f` with the process' stdout file descriptor redirected into a temporary
/// file and return the raw bytes that were written.
///
/// Both libraries print through the process' shared C `stdout`, so capturing at
/// the file-descriptor level (rather than via Rust's `print!` machinery) is what
/// actually observes their output. `fflush(NULL)` is issued on both sides of the
/// redirect so nothing leaks across the boundary.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;

    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!(
        "staticloop-stdout-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut tmp = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create stdout capture file");

    let result;
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 failed");

        result = f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
    }

    tmp.seek(SeekFrom::Start(0)).expect("rewind capture file");
    let mut bytes = Vec::new();
    tmp.read_to_end(&mut bytes).expect("read capture file");
    drop(tmp);
    let _ = std::fs::remove_file(&tmp_path);

    (result, bytes)
}

/// Interesting `int` values: small magnitudes, powers of two, and the values
/// around the signed 32-bit boundaries that trigger wrapping in the running sum.
pub fn interesting_i32() -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        7,
        -7,
        10,
        -10,
        100,
        -100,
        12345,
        -12345,
        65535,
        65536,
        -65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 30,
        -(1 << 30),
        214748364,
        -214748364,
    ];
    for bit in 0..31 {
        v.push(1i32 << bit);
        v.push(-(1i32 << bit));
    }
    v
}
