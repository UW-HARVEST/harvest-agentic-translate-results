//! Build the artifacts the differential tests compare.
//!
//! Nothing under `c_src/` is modified: the C source is only read, and every
//! output goes to `OUT_DIR`.
//!
//! Produced artifacts:
//!   * `c_driver`      — C executable, same as `add_executable` in c_src/CMakeLists.txt
//!   * `libc_driver.so`— C shared library (`-shared -fPIC`), exports `main`
//!   * `librust_driver.so` — Rust cdylib built from `src/lib.rs`, must export `main`
//!
//! The Rust *executable* is not built here; integration tests get it from
//! `CARGO_BIN_EXE_driver`, which is exactly the binary the crate ships.

use std::path::{Path, PathBuf};
use std::process::Command;

fn run(what: &str, cmd: &mut Command) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    if !out.status.success() {
        panic!(
            "{what} failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let c_main = manifest.join("c_src/src/main.c");

    println!("cargo:rerun-if-changed={}", c_main.display());
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/hello.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    // ---- C executable (mirrors c_src/CMakeLists.txt: add_executable) ----
    let c_exe = out_dir.join("c_driver");
    run(
        "cc (C executable)",
        Command::new(&cc)
            .arg("-O2")
            .arg("-o")
            .arg(&c_exe)
            .arg(&c_main),
    );

    // ---- C shared library: the dlopen()-able form of the same source ----
    let c_so = out_dir.join("libc_driver.so");
    run(
        "cc (C shared library)",
        Command::new(&cc)
            .args(["-shared", "-fPIC", "-O2", "-o"])
            .arg(&c_so)
            .arg(&c_main),
    );

    // ---- Rust cdylib built straight from src/lib.rs ----
    // Built with rustc directly (rather than relying on cargo emitting the
    // cdylib during `cargo test`) so the tests always have a `.so` to dlopen.
    // `-C panic=abort` matches the shipped `[profile.release]`.
    let rust_so = out_dir.join("librust_driver.so");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    run(
        "rustc (Rust cdylib)",
        Command::new(&rustc)
            .args(["--crate-type", "cdylib", "--edition", "2021"])
            .args(["--crate-name", "rust_driver"])
            .args(["-O", "-C", "panic=abort"])
            .arg("-o")
            .arg(&rust_so)
            .arg(manifest.join("src/lib.rs")),
    );

    for (var, path) in [
        ("C_DRIVER_EXE", &c_exe),
        ("C_DRIVER_SO", &c_so),
        ("RUST_DRIVER_SO", &rust_so),
    ] {
        assert!(
            Path::new(path).exists(),
            "{var} artifact missing: {}",
            path.display()
        );
        println!("cargo:rustc-env={var}={}", path.display());
    }
}
