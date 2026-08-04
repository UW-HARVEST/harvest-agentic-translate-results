// Build script: when building tests, also produce the `cdylib` form of
// this crate (with the `export_main` feature) into a sibling target
// directory. The integration tests load that .so via libloading and
// compare it against the C library.
//
// We use a SEPARATE target directory (`target-cdylib`) so cargo's file
// locks on the main target directory do not deadlock with this nested
// build invocation.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Only run during integration test builds (i.e., when the `test`
    // profile is active). Heuristic: re-run unconditionally is safe;
    // cargo invalidates this script when sources change anyway.
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // If we are recursively invoked by the inner cargo build, skip.
    if std::env::var("DRIVER_BUILDING_CDYLIB").is_ok() {
        return;
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target-cdylib");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(&cargo);
    cmd.arg("build")
        .arg("--lib")
        .arg("--features")
        .arg("export_main")
        .arg("--target-dir")
        .arg(&target_dir)
        .env("DRIVER_BUILDING_CDYLIB", "1");

    let status = cmd.status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("inner cargo build for cdylib failed with status: {s}"),
        Err(e) => panic!("failed to spawn inner cargo build: {e}"),
    }

    // Expose the resolved path to integration tests.
    println!(
        "cargo:rustc-env=DRIVER_RUST_CDYLIB_PATH={}",
        target_dir.join("debug").join("libdriver.so").display()
    );
}
