//! Shared helpers: locate and load the C and Rust shared libraries.
//!
//! Each integration test binary compiles this module separately and uses only
//! part of it, hence the blanket `dead_code` allowance.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

unsafe extern "C" {
    pub fn free(p: *mut c_void);
}

pub type ExtractFilenameFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
pub type CreateFilenameFn = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().expect("workspace root").to_path_buf()
}

/// Build `c_src` with CMake if the shared library is not there yet.
pub fn c_lib_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let so = build.join("libdriver.so");
        if !so.exists() {
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let cfg = std::process::Command::new("cmake")
                .current_dir(&build)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .output()
                .expect("run cmake");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let b = std::process::Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .output()
                .expect("run cmake --build");
            assert!(
                b.status.success(),
                "cmake build failed:\n{}",
                String::from_utf8_lossy(&b.stderr)
            );
        }
        assert!(so.exists(), "C shared library missing: {}", so.display());
        so
    })
}

/// Build the Rust `cdylib` and return its path.
///
/// `cargo test` does not emit the `cdylib` artifact for the crate under test,
/// so the tests build it themselves. A dedicated `--target-dir` is used so the
/// nested `cargo` invocation does not block on the outer build lock.
///
/// The nested build inherits the feature selection of the test run through the
/// `DRIVER_SO_FEATURE_ARGS` environment variable (set by `run-tests.sh`),
/// defaulting to the crate's default features.
pub fn rust_lib_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let manifest = manifest_dir();
        let target_dir = manifest.join("target").join("ffi-so");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(&manifest)
            .arg("build")
            .arg("--lib")
            .arg("--manifest-path")
            .arg(manifest.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target_dir);
        if let Ok(extra) = std::env::var("DRIVER_SO_FEATURE_ARGS") {
            for a in extra.split_whitespace() {
                cmd.arg(a);
            }
        }
        let out = cmd.output().expect("run cargo build --lib");
        assert!(
            out.status.success(),
            "nested cargo build failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let so = target_dir.join("debug").join("libdriver.so");
        assert!(so.exists(), "Rust shared library missing: {}", so.display());
        so
    })
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn load() -> Libs {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        unsafe {
            Libs {
                c: Library::new(cp).expect("load C .so"),
                rust: Library::new(rp).expect("load Rust .so"),
            }
        }
    }

    pub fn extract_filename(&self) -> (Symbol<'_, ExtractFilenameFn>, Symbol<'_, ExtractFilenameFn>) {
        unsafe {
            (
                self.c.get(b"extractFilename\0").expect("C extractFilename"),
                self.rust
                    .get(b"extractFilename\0")
                    .expect("Rust extractFilename"),
            )
        }
    }

    pub fn create_filename(&self) -> (Symbol<'_, CreateFilenameFn>, Symbol<'_, CreateFilenameFn>) {
        unsafe {
            (
                self.c
                    .get(b"FIO_createFilename_fromOutDir\0")
                    .expect("C FIO_createFilename_fromOutDir"),
                self.rust
                    .get(b"FIO_createFilename_fromOutDir\0")
                    .expect("Rust FIO_createFilename_fromOutDir"),
            )
        }
    }
}

/// A NUL-terminated byte buffer with one deterministic "guard" byte placed
/// *before* the string, so that the original C code's read of
/// `outDirName[strlen(outDirName)-1]` on an empty string is well defined and
/// identical for both implementations.
pub struct GuardedCStr {
    buf: Vec<u8>,
}

impl GuardedCStr {
    pub fn new(guard: u8, s: &[u8]) -> GuardedCStr {
        assert!(!s.contains(&0), "interior NUL not allowed");
        let mut buf = Vec::with_capacity(s.len() + 2);
        buf.push(guard);
        buf.extend_from_slice(s);
        buf.push(0);
        GuardedCStr { buf }
    }

    /// Pointer to the first byte of the string itself (after the guard byte).
    pub fn ptr(&self) -> *const c_char {
        unsafe { self.buf.as_ptr().add(1) as *const c_char }
    }
}
