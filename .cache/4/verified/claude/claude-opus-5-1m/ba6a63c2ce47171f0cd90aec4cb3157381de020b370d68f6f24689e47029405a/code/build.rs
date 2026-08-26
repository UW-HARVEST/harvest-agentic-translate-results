//! Build script.
//!
//! Its only job is to make the differential tests self-contained: it compiles
//! the *unmodified* C ground truth (`c_src/src/container_of.c`) into a shared
//! library inside `OUT_DIR` and tells the rest of the crate where to find it via
//! `cargo:rustc-env`. Nothing inside `c_src/` is touched -- all output goes to
//! `OUT_DIR`.
//!
//! Two variants are produced:
//!
//! * `C_SO_PATH`     -- compiled with the same flags CMake uses for the default
//!                      (empty `CMAKE_BUILD_TYPE`) configuration plus
//!                      `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`, i.e. `-fPIC`
//!                      and no `-O` flag. This is the canonical reference.
//! * `C_SO_PATH_O2`  -- the same source at `-O2`, used by one extra test that
//!                      confirms the optimiser does not change the observable
//!                      behaviour of the reference.
//! * `C_EXE_PATH`    -- the C ground truth linked as an executable, so the
//!                      end-to-end test always has a reference program even if
//!                      the CMake build tree has not been created. When
//!                      `c_src/build/driver` (the CMake output) exists, the test
//!                      prefers that one.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let c_source = manifest_dir.join("c_src").join("src").join("container_of.c");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", c_source.display());
    println!("cargo:rerun-if-changed=build.rs");

    // Only meaningful on unix-likes, which is all the C code targets.
    if !c_source.exists() {
        panic!("missing C ground truth at {}", c_source.display());
    }

    let default_so = out_dir.join("libcdriver.so");
    let o2_so = out_dir.join("libcdriver_o2.so");
    let exe = out_dir.join("cdriver");

    compile(&c_source, &default_so, &["-shared", "-fPIC"]);
    compile(&c_source, &o2_so, &["-shared", "-fPIC", "-O2"]);
    compile(&c_source, &exe, &["-fPIC", "-pie"]);

    println!("cargo:rustc-env=C_SO_PATH={}", default_so.display());
    println!("cargo:rustc-env=C_SO_PATH_O2={}", o2_so.display());
    println!("cargo:rustc-env=C_EXE_PATH={}", exe.display());

    build_rust_fallback_cdylib(&manifest_dir, &out_dir);
}

/// `cargo test` does not build the `cdylib` target: no test target depends on it,
/// so cargo has no reason to. The differential tests therefore prefer
/// `target/<profile>/libdriver.so` (the real cargo artifact, produced by
/// `cargo build`) and fall back to this copy, compiled straight from the same
/// `src/lib.rs` with the same `rustc`, so that a bare `cargo test` on a fresh
/// checkout still exercises a genuine shared object with genuine `#[no_mangle]`
/// exports instead of failing on a missing file.
fn build_rust_fallback_cdylib(manifest_dir: &Path, out_dir: &Path) {
    let lib_rs = manifest_dir.join("src").join("lib.rs");
    println!("cargo:rerun-if-changed={}", lib_rs.display());
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src").join("container_of.rs").display()
    );

    let output = out_dir.join("libdriver.so");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());

    let status = Command::new(&rustc)
        .arg("--edition=2021")
        .arg("--crate-type=cdylib")
        .arg("--crate-name=driver")
        .arg("-o")
        .arg(&output)
        .arg(&lib_rs)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {rustc}: {e}"));

    if !status.success() {
        panic!("{rustc} failed to build the fallback cdylib: {status}");
    }

    println!("cargo:rustc-env=RUST_SO_FALLBACK={}", output.display());
}

fn compile(source: &Path, output: &Path, extra: &[&str]) {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let mut cmd = Command::new(&cc);
    cmd.args(extra);
    cmd.arg("-o").arg(output).arg(source);

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));

    if !status.success() {
        panic!("{cc} failed to build {}: {status}", output.display());
    }
}
