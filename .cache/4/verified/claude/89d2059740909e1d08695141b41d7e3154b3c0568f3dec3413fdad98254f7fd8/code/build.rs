//! Builds the reference C code so the differential tests can load it.
//!
//! Two artifacts are produced in `OUT_DIR` (nothing under `c_src/` is touched):
//!   * `libdriver_c.so` -- `main.c` + `inventory.c` as a shared library, whose
//!     exported symbols are compared against this crate's `cdylib`;
//!   * `driver_c`       -- the same sources as an executable, for end-to-end
//!     stdin/stdout comparison against the `driver` binary.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let c_src = manifest_dir.join("c_src");

    let sources = [c_src.join("src/main.c"), c_src.join("src/inventory.c")];
    let include = c_src.join("include");

    println!("cargo:rerun-if-changed={}", sources[0].display());
    println!("cargo:rerun-if-changed={}", sources[1].display());
    println!(
        "cargo:rerun-if-changed={}",
        include.join("inventory.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        include.join("generic_containers.h").display()
    );

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let so_path = out_dir.join("libdriver_c.so");
    let mut cmd = Command::new(&cc);
    cmd.arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(&so_path)
        .args(&sources)
        .arg("-I")
        .arg(&include);
    run(cmd, "building libdriver_c.so");

    let exe_path = out_dir.join("driver_c");
    let mut cmd = Command::new(&cc);
    cmd.arg("-o")
        .arg(&exe_path)
        .args(&sources)
        .arg("-I")
        .arg(&include);
    run(cmd, "building driver_c");

    println!("cargo:rustc-env=C_SO_PATH={}", so_path.display());
    println!("cargo:rustc-env=C_EXE_PATH={}", exe_path.display());
}

fn run(mut cmd: Command, what: &str) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn compiler while {what}: {e}"));
    assert!(status.success(), "{what} failed: {status}");
}
