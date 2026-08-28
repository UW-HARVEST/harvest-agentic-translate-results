//! Shared helpers for locating and loading the two shared libraries under
//! comparison: the reference C build and the Rust `cdylib`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Directory holding the cargo build artifacts for the current profile
/// (e.g. `target/debug`). Derived from the running test executable, which
/// lives in `<profile>/deps/`.
pub fn artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // <profile>/deps/<test-binary>
    exe.parent()
        .and_then(Path::parent)
        .expect("artifact dir")
        .to_path_buf()
}

/// Path to the Rust `cdylib` produced from `src/lib.rs`.
///
/// `[lib] name = "get_predict_func_lib"`, so the artifact is
/// `libget_predict_func_lib.so`.
///
/// Because the crate declares `crate-type = ["cdylib"]` only, cargo does *not*
/// build the shared object as a side effect of `cargo test` (integration tests
/// have nothing linkable to depend on). Build it explicitly, once per test
/// binary, using the same profile the tests are running under.
pub fn rust_so() -> PathBuf {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        if let Ok(p) = std::env::var("RUST_SO_PATH") {
            return PathBuf::from(p);
        }
        if let Some(p) = find_rust_so() {
            return p;
        }
        build_cdylib();
        find_rust_so().unwrap_or_else(|| {
            let dir = artifact_dir();
            panic!(
                "Rust cdylib still not found in {} after `cargo build --lib`. Contents: {:?}",
                dir.display(),
                std::fs::read_dir(&dir)
                    .map(|rd| rd
                        .filter_map(|e| e.ok().map(|e| e.file_name()))
                        .collect::<Vec<_>>())
                    .unwrap_or_default()
            )
        })
    })
    .clone()
}

fn find_rust_so() -> Option<PathBuf> {
    let dir = artifact_dir();
    let candidates = [
        "libget_predict_func_lib.so",
        "libget_predict_func_lib.dylib",
        "get_predict_func_lib.dll",
    ];
    candidates
        .iter()
        .map(|name| dir.join(name))
        .find(|p| p.exists())
}

/// Run `cargo build --lib` for the profile the tests were compiled with, so the
/// `cdylib` lands next to the test binaries.
fn build_cdylib() {
    let profile_dir = artifact_dir();
    let profile = profile_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "debug".to_string());

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build").arg("--lib");
    if profile == "release" {
        cmd.arg("--release");
    } else if profile != "debug" {
        cmd.arg("--profile").arg(&profile);
    }
    // Mirror the feature selection the tests were built with, when the caller
    // supplies it (the crate currently declares no features, but keep the
    // plumbing so feature sweeps stay honest).
    if std::env::var("RUST_SO_NO_DEFAULT_FEATURES").is_ok() {
        cmd.arg("--no-default-features");
    }
    if let Ok(f) = std::env::var("RUST_SO_FEATURES") {
        if !f.is_empty() {
            cmd.arg("--features").arg(f);
        }
    }
    // Avoid inheriting the parent cargo's per-invocation environment, which can
    // confuse a nested build.
    for k in [
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS",
        "CARGO_MAKEFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        cmd.env_remove(k);
    }

    let out = cmd.output().expect("spawn cargo build --lib");
    assert!(
        out.status.success(),
        "cargo build --lib failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Path to the reference C shared library built from `c_src/`.
///
/// The CMake project name is derived from the *parent* directory name of
/// `c_src`, so the file name varies per checkout; glob for any `.so` in the
/// build directory instead of hard-coding it.
pub fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let build = root.join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for entry in rd.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("lib") && (name.ends_with(".so") || name.ends_with(".dylib")) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}. Build it with cmake first.",
            build.display()
        )
    })
}
