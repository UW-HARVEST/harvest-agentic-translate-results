//! Shared helpers for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported symbols, so the `#[no_mangle]` export
//! wrappers are part of what gets exercised. The Rust code under test is never
//! called directly from the test binary.
//!
//! Each invocation runs in a child process (`examples/ffi_runner.rs`) so that
//! the library's stdout can be captured byte-for-byte without interference from
//! the test harness, which writes to stdout on other threads.

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Cargo features enabled for this test run.
///
/// The crate currently declares no `[features]`, so this is empty. It is the
/// single place to extend if features are ever added, so the on-demand builds
/// below stay in sync with the feature set the test binary was compiled with.
fn features_in_effect() -> Vec<&'static str> {
    Vec::new()
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn profile_dir() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// Builds the crate's `cdylib` and the `ffi_runner` example once per test
/// process.
///
/// `cargo test` does not build the `cdylib` target on its own, since no test
/// target links against it. A dedicated target directory is used so this nested
/// `cargo` invocation does not contend for the lock held by the outer
/// `cargo test`; rebuilding also means the `.so` can never be stale relative to
/// `src/`.
fn build_artifacts() -> &'static (PathBuf, PathBuf) {
    static ARTIFACTS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let target_dir = manifest_dir().join("target/ffi-cdylib");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let mut cmd = Command::new(cargo);
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--lib")
            .args(["--example", "ffi_runner"])
            .arg("--no-default-features")
            .args(features_in_effect().iter().flat_map(|f| ["--features", f]))
            .arg("--target-dir")
            .arg(&target_dir);
        if !cfg!(debug_assertions) {
            cmd.arg("--release");
        }

        let output = cmd.output().expect("failed to spawn `cargo build`");
        assert!(
            output.status.success(),
            "`cargo build` for the cdylib/runner failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let base = target_dir.join(profile_dir());
        let so = base.join("libhello.so");
        let runner = base.join("examples/ffi_runner");
        assert!(so.is_file(), "Rust cdylib not found at {}", so.display());
        assert!(runner.is_file(), "runner not found at {}", runner.display());
        (so, runner)
    })
}

/// Path of the C shared library built from `c_src/`.
pub fn c_lib_path() -> PathBuf {
    let path = manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libhello.so");
    assert!(
        path.is_file(),
        "C shared library missing at {}; build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        path.display()
    );
    path
}

/// Path of the Rust `cdylib` for this crate.
pub fn rust_lib_path() -> PathBuf {
    build_artifacts().0.clone()
}

/// Everything one invocation of `helloworld` produced.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Return value of each call, in order.
    pub rets: Vec<c_int>,
    /// Exact bytes the library wrote to standard output.
    pub stdout: Vec<u8>,
}

/// Loads `lib_path` in a child process, calls the exported `helloworld` symbol
/// `calls` times, and returns the return values plus the emitted stdout bytes.
pub fn run_helloworld(lib_path: &Path, calls: usize) -> Outcome {
    let runner = &build_artifacts().1;
    let output = Command::new(runner)
        .arg(lib_path)
        .arg(calls.to_string())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", runner.display()));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "runner failed for {} ({} call(s)): {}\n{stderr}",
        lib_path.display(),
        calls,
        output.status
    );

    let line = stderr
        .lines()
        .find_map(|l| l.strip_prefix("RETS:"))
        .unwrap_or_else(|| panic!("runner did not report return values:\n{stderr}"));
    let rets: Vec<c_int> = if line.is_empty() {
        Vec::new()
    } else {
        line.split(',')
            .map(|s| s.parse().expect("integer return value"))
            .collect()
    };
    assert_eq!(rets.len(), calls, "runner reported the wrong call count");

    Outcome {
        rets,
        stdout: output.stdout,
    }
}

/// Convenience wrapper for a single call.
pub fn outcome(lib_path: &Path) -> Outcome {
    run_helloworld(lib_path, 1)
}
