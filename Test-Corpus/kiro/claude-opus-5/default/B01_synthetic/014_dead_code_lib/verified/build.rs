//! Builds the crate as a `cdylib` for the differential tests.
//!
//! `cargo test` compiles the library only as an rlib/rmeta, so no `libdriver.so`
//! is produced for the test profile. The integration tests must load a *fresh*
//! shared object through `libloading`, so we link one here from the same
//! sources, with the same feature set, and hand its path to the tests via an
//! environment variable.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=src/lib.rs");
    println!("cargo::rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let so_path = out_dir.join("libdriver_under_test.so");

    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let mut cmd = Command::new(rustc);
    cmd.arg("--crate-type=cdylib")
        .arg("--crate-name=driver")
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=0")
        .arg("--cfg")
        .arg("driver_cdylib_under_test")
        .arg("src/lib.rs")
        .arg("-o")
        .arg(&so_path);

    // Mirror the features cargo enabled for this build.
    for (key, _) in env::vars() {
        if let Some(feature) = key.strip_prefix("CARGO_FEATURE_") {
            let name = feature.to_lowercase();
            cmd.arg("--cfg").arg(format!("feature=\"{name}\""));
            let dashed = name.replace('_', "-");
            if dashed != name {
                cmd.arg("--cfg").arg(format!("feature=\"{dashed}\""));
            }
        }
    }

    let status = cmd.status().expect("failed to spawn rustc for the cdylib");
    assert!(status.success(), "rustc failed to build the test cdylib");

    println!("cargo::rustc-env=DRIVER_RUST_SO={}", so_path.display());
}
