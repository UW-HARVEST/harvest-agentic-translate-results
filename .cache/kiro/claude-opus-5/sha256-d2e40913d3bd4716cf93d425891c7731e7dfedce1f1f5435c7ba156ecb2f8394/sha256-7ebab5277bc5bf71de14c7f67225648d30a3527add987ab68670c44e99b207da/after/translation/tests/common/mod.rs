#![allow(dead_code)]

//! Shared helpers: locate and load the C and Rust shared libraries.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Signature of the single public entry point.
pub type Float2Half = unsafe extern "C" fn(f32) -> u16;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("manifest has a parent").to_path_buf()
}

fn find_in_dir(dir: &Path, pred: &dyn Fn(&str) -> bool) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if p.is_file() && pred(&name) {
            return Some(p);
        }
    }
    None
}

/// Path to the C shared library produced by the CMake build.
pub fn c_lib_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    find_in_dir(&build, &|n: &str| n.starts_with("lib") && n.ends_with(".so"))
        .unwrap_or_else(|| {
            panic!(
                "C shared library not found in {}. Build it with:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
}

/// Path to the Rust cdylib for the profile the tests were built under.
///
/// `cargo test` does not build the `cdylib` artifact for this package (it
/// builds an rlib for the test harness instead), so if the artifact is not
/// present we build it on demand into a side target directory. The build uses
/// the same profile and feature selection as the test run so that the .so we
/// exercise matches the configuration under test.
pub fn rust_lib_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        // current_exe = <target-dir>/<profile>/deps/<test-bin>
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("profile dir")
            .to_path_buf();

        if let Some(p) = find_cdylib(&profile_dir) {
            return p;
        }

        // Build it ourselves into a nested target dir to avoid contending on
        // the target-directory lock held by the outer `cargo test`.
        let profile_name = profile_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "debug".to_string());
        let side_target = profile_dir.join("ffi-cdylib");

        let mut cmd = Command::new(env!("CARGO"));
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
            .arg("build")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&side_target);
        if profile_name == "release" {
            cmd.arg("--release");
        }
        apply_feature_flags(&mut cmd);

        let out = cmd.output().expect("spawn cargo build for cdylib");
        assert!(
            out.status.success(),
            "cargo build of the cdylib failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        let built = side_target.join(&profile_name);
        find_cdylib(&built).unwrap_or_else(|| {
            panic!("cdylib still not found in {}", built.display())
        })
    })
    .clone()
}

fn find_cdylib(dir: &Path) -> Option<PathBuf> {
    find_in_dir(dir, &|n: &str| n == "libfloat2half_lib.so").or_else(|| {
        find_in_dir(dir, &|n: &str| {
            n.starts_with("libfloat2half_lib") && n.ends_with(".so")
        })
    })
}

/// Mirror the feature selection of the current test build onto the nested
/// cargo invocation, using the `CARGO_FEATURE_*` variables cargo exports.
fn apply_feature_flags(cmd: &mut Command) {
    let mut features: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_").map(|f| f.to_lowercase().replace('_', "-"))
        })
        .collect();
    features.sort();
    features.dedup();

    // The crate currently declares no features; only pass flags when a
    // feature-based configuration is actually in play, or when explicitly
    // requested via TEST_FEATURES / TEST_NO_DEFAULT_FEATURES.
    if let Ok(explicit) = std::env::var("TEST_FEATURES") {
        cmd.arg("--no-default-features");
        if !explicit.trim().is_empty() {
            cmd.arg("--features").arg(explicit);
        }
        return;
    }
    if std::env::var_os("TEST_NO_DEFAULT_FEATURES").is_some() {
        cmd.arg("--no-default-features");
        if !features.is_empty() {
            cmd.arg("--features").arg(features.join(","));
        }
    }
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn load() -> Self {
        unsafe {
            let c = Library::new(c_lib_path()).expect("load C .so");
            let rust = Library::new(rust_lib_path()).expect("load Rust .so");
            Libs { c, rust }
        }
    }

    pub fn c_float2half(&self) -> Symbol<'_, Float2Half> {
        unsafe {
            self.c
                .get(b"float2half\0")
                .expect("C .so must export float2half")
        }
    }

    pub fn rust_float2half(&self) -> Symbol<'_, Float2Half> {
        unsafe {
            self.rust
                .get(b"float2half\0")
                .expect("Rust .so must export float2half")
        }
    }
}
