// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Builds the *reference* C artifacts that the differential tests compare
// against. Nothing under `c_src/` is modified: the sources are only read, and
// every output lands in `OUT_DIR`.
//
// Two artifacts are produced from `c_src/src/main.c`:
//
//   * `libc_driver.so` - the translation unit built with `-shared -fPIC`, so it
//     exports `driver` and `main` and can be `dlopen`ed by the FFI tests.
//   * `c_driver`       - the executable, matching what `c_src/CMakeLists.txt`
//     builds, used by the process-level tests.
//
// Their paths are handed to the tests through `cargo:rustc-env`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let c_main = manifest_dir.join("c_src").join("src").join("main.c");

    println!("cargo:rerun-if-changed={}", c_main.display());
    println!("cargo:rerun-if-changed=build.rs");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    // The C source uses digraphs (`%:`, `<%`, `%>`), which are standard C but
    // are only recognised outside of "traditional" preprocessing modes. The
    // default `gnu17` dialect handles them, so no extra flags are needed beyond
    // what CMakeLists.txt uses.
    let so_path = out_dir.join("libc_driver.so");
    compile(
        &cc,
        &["-shared", "-fPIC", "-O2"],
        &c_main,
        &so_path,
        "C shared library",
    );

    let exe_path = out_dir.join("c_driver");
    compile(&cc, &["-O2"], &c_main, &exe_path, "C executable");

    // A second, unoptimised executable. `c_src/CMakeLists.txt` builds with no
    // explicit optimisation level, so comparing `-O0` against `-O2` proves the C
    // reference behaviour is not optimisation-dependent (i.e. that the tests are
    // pinned to well-defined behaviour rather than to whatever one particular
    // build happened to emit).
    let exe_o0_path = out_dir.join("c_driver_O0");
    compile(&cc, &["-O0"], &c_main, &exe_o0_path, "C executable (-O0)");

    println!("cargo:rustc-env=C_DRIVER_SO={}", so_path.display());
    println!("cargo:rustc-env=C_DRIVER_EXE={}", exe_path.display());
    println!("cargo:rustc-env=C_DRIVER_EXE_O0={}", exe_o0_path.display());
}

fn compile(cc: &str, flags: &[&str], src: &Path, out: &Path, what: &str) {
    let status = Command::new(cc)
        .args(flags)
        .arg(src)
        .arg("-o")
        .arg(out)
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke the C compiler {cc:?}: {e}"));
    assert!(status.success(), "failed to build the {what} from {src:?}");
}
