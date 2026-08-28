//! Build script for the differential test-harness.
//!
//! It does two things, both of which only affect *tests*:
//!
//! 1. Configures and builds the original C library (`../c_src`) as a shared
//!    object, out-of-tree (inside `OUT_DIR`, so nothing under `c_src/` is
//!    modified), and exports its absolute path as the `C_SO_PATH` env var.
//! 2. Computes the path where cargo will place this crate's own `cdylib` and
//!    exports it as `RUST_SO_DIR`, so the tests can `dlopen` the Rust library
//!    exactly the way an external C consumer would.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let c_src = manifest_dir
        .parent()
        .expect("crate dir has a parent")
        .join("c_src");

    println!("cargo:rerun-if-changed={}", c_src.join("src/lib.c").display());
    println!(
        "cargo:rerun-if-changed={}",
        c_src.join("include/lib.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        c_src.join("CMakeLists.txt").display()
    );
    println!("cargo:rerun-if-env-changed=C_SO_PATH");

    // `target/<profile>/build/<pkg>-<hash>/out` -> `target/<profile>`
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR has >= 3 ancestors")
        .to_path_buf();
    println!("cargo:rustc-env=RUST_SO_DIR={}", profile_dir.display());

    // The test suite scales its exhaustive sweeps by optimisation level. It
    // cannot use `cfg!(debug_assertions)` for that, because `[profile.dev]`
    // deliberately turns debug assertions off (see Cargo.toml).
    println!(
        "cargo:rustc-env=HARVEST_OPT_LEVEL={}",
        std::env::var("OPT_LEVEL").unwrap_or_else(|_| "0".to_string())
    );

    // Guaranteed fallback copy of this crate's own cdylib.
    //
    // `cargo test` does not emit the `cdylib` artifact for a `crate-type =
    // ["cdylib"]` package (integration tests do not link it), so if the tests
    // are run without a preceding `cargo build` there would be no Rust `.so` to
    // `dlopen`. Compile one here from the very same `src/lib.rs` so the test
    // suite is self-contained. `run_all_feature_combos.sh` always runs
    // `cargo build` first, and the harness prefers cargo's own artifact.
    build_fallback_cdylib(&manifest_dir, &out_dir);
    println!("cargo:rerun-if-changed={}", manifest_dir.join("src/lib.rs").display());

    // Allow an explicit override (used by the multi-profile runner script).
    if let Ok(p) = std::env::var("C_SO_PATH") {
        if !p.is_empty() {
            println!("cargo:rustc-env=C_SO_PATH={p}");
            return;
        }
    }

    let c_build = out_dir.join("c_build");
    std::fs::create_dir_all(&c_build).expect("create c_build dir");

    let status = Command::new("cmake")
        .arg("-S")
        .arg(&c_src)
        .arg("-B")
        .arg(&c_build)
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .status()
        .expect("failed to spawn `cmake` (is cmake installed?)");
    assert!(status.success(), "cmake configure failed");

    let status = Command::new("cmake")
        .arg("--build")
        .arg(&c_build)
        .status()
        .expect("failed to spawn `cmake --build`");
    assert!(status.success(), "cmake build failed");

    let so = find_shared_object(&c_build)
        .unwrap_or_else(|| panic!("no .so produced under {}", c_build.display()));
    println!("cargo:rustc-env=C_SO_PATH={}", so.display());
}

fn build_fallback_cdylib(manifest_dir: &Path, out_dir: &Path) {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let opt_level = std::env::var("OPT_LEVEL").unwrap_or_else(|_| "0".to_string());
    let dest = out_dir.join("fallback");
    std::fs::create_dir_all(&dest).expect("create fallback dir");

    let mut cmd = Command::new(rustc);
    cmd.arg("--crate-type=cdylib")
        .arg("--crate-name=synth_pair_lib")
        .arg("--edition=2021")
        .arg(format!("-Copt-level={opt_level}"))
        .arg("-Cpanic=abort")
        .arg(manifest_dir.join("src/lib.rs"))
        .arg("--out-dir")
        .arg(&dest);
    // Mirror the feature flags cargo passes for the current feature selection.
    for (k, v) in std::env::vars() {
        if let Some(feat) = k.strip_prefix("CARGO_FEATURE_") {
            if v == "1" {
                let feat = feat.to_ascii_lowercase().replace('_', "-");
                cmd.arg("--cfg").arg(format!("feature=\"{feat}\""));
            }
        }
    }
    let status = cmd.status().expect("failed to spawn rustc for fallback cdylib");
    assert!(status.success(), "fallback cdylib build failed");
    println!(
        "cargo:rustc-env=FALLBACK_RUST_SO={}",
        dest.join("libsynth_pair_lib.so").display()
    );
}

/// The C project name is derived from the *parent directory name* by
/// `CMakeLists.txt`, so the produced file name is not knowable up front; glob
/// for it instead.
fn find_shared_object(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for entry in std::fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        if path.is_file() {
            let name = path.file_name()?.to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                // Prefer the shortest name (no version suffixes).
                if best
                    .as_ref()
                    .map(|b| name.len() < b.file_name().unwrap().to_string_lossy().len())
                    .unwrap_or(true)
                {
                    best = Some(path);
                }
            }
        }
    }
    best
}
