// Build script for the differential test suite.
//
// `cargo test` does NOT rebuild a `cdylib`-only library target (nothing in the
// test graph links against it), so integration tests that `dlopen`
// `target/<profile>/libmodeselect_lib.so` would happily test a STALE artifact
// and report false passes. To make the differential tests trustworthy this
// script rebuilds, on every change of the sources:
//
//   * the C reference shared library (via CMake, falling back to a direct `cc`
//     invocation), and
//   * the Rust shared library, twice: unoptimised and optimised, so the
//     translation is checked to be optimisation-independent as well.
//
// The resulting paths are handed to the tests through `env!()`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    println!("cargo::rerun-if-changed=src/lib.rs");
    println!("cargo::rerun-if-changed=c_src/src/lib.c");
    println!("cargo::rerun-if-changed=c_src/include/lib.h");
    println!("cargo::rerun-if-changed=c_src/CMakeLists.txt");
    println!("cargo::rerun-if-changed=build.rs");

    let c_so = build_c(&manifest, &out);
    println!("cargo::rustc-env=C_SO_PATH={}", c_so.display());

    let rust_so = build_rust(&manifest, &out, 0, "libmodeselect_lib_dbg.so");
    println!("cargo::rustc-env=RUST_SO_PATH={}", rust_so.display());

    let rust_so_opt = build_rust(&manifest, &out, 3, "libmodeselect_lib_opt.so");
    println!("cargo::rustc-env=RUST_SO_OPT_PATH={}", rust_so_opt.display());
}

/// Build `c_src` as a shared library. Nothing inside `c_src/` is modified: the
/// CMake build tree lives in `OUT_DIR`.
fn build_c(manifest: &Path, out: &Path) -> PathBuf {
    let src_dir = manifest.join("c_src");
    let build_dir = out.join("cbuild");
    std::fs::create_dir_all(&build_dir).unwrap();

    let cmake_ok = Command::new("cmake")
        .arg("-S")
        .arg(&src_dir)
        .arg("-B")
        .arg(&build_dir)
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

    if cmake_ok {
        if let Some(p) = find_so(&build_dir) {
            return p;
        }
    }

    // Fallback: plain cc, matching CMake's default (no CMAKE_BUILD_TYPE => no
    // optimisation flags).
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let dst = out.join("libc_reference.so");
    let status = Command::new(&cc)
        .arg("-shared")
        .arg("-fPIC")
        .arg("-I")
        .arg(src_dir.join("include"))
        .arg(src_dir.join("src/lib.c"))
        .arg("-o")
        .arg(&dst)
        .status()
        .expect("failed to run the C compiler");
    assert!(status.success(), "compiling the C reference library failed");
    dst
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            // prefer the canonical name if several exist
            if p.file_name().map(|n| n == "libtranslated_rust.so").unwrap_or(false) {
                return Some(p);
            }
            best = Some(p);
        }
    }
    best
}

/// Compile `src/lib.rs` as a standalone `cdylib` (it has no crate
/// dependencies), so the tests always exercise the current source.
fn build_rust(manifest: &Path, out: &Path, opt_level: u8, name: &str) -> PathBuf {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let dst = out.join(name);
    let status = Command::new(&rustc)
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("cdylib")
        .arg("--crate-name")
        .arg("modeselect_lib")
        .arg("-C")
        .arg(format!("opt-level={opt_level}"))
        .arg("-C")
        .arg("debug-assertions=on")
        .arg("--cap-lints")
        .arg("allow")
        .arg(manifest.join("src/lib.rs"))
        .arg("-o")
        .arg(&dst)
        .status()
        .expect("failed to run rustc");
    assert!(status.success(), "building the Rust cdylib ({name}) failed");
    dst
}
