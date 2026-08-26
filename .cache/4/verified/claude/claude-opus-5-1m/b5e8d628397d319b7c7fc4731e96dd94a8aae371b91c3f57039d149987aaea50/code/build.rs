//! Compiles the ground-truth C translation unit (`c_src/src/main.c`) into
//! shared objects and executables so the differential test-suite can dlopen /
//! spawn it next to the Rust artefacts.
//!
//! Two optimisation levels are produced, because the C program's arithmetic
//! relies on signed overflow, negative `<<` and negative `>>`: `-O0` (what
//! `c_src/CMakeLists.txt` produces, since it sets no `CMAKE_BUILD_TYPE`) and
//! `-O2`. Every differential test runs against both.
//!
//! Nothing in `c_src/` is modified: every artefact lands in `OUT_DIR`.
//!
//! If no C compiler is available, this script only warns: the Rust crate itself
//! must still build. The paths are exported either way, so the tests compile and
//! fail with a clear "shared object is missing" message instead.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let c_source = manifest_dir.join("c_src/src/main.c");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", c_source.display());
    println!("cargo:rerun-if-changed=build.rs");

    let have_source = c_source.exists();
    if !have_source {
        println!(
            "cargo:warning=missing {} - the differential tests will not run",
            c_source.display()
        );
    }

    for opt in ["0", "2"] {
        let so = out_dir.join(format!("libc_driver_O{opt}.so"));
        let exe = out_dir.join(format!("c_driver_O{opt}"));

        if have_source {
            // `-shared -fPIC`: exposes `array`, `main` and
            // `perform_expensive_operations` exactly as the translation unit
            // declares them.
            compile(
                &c_source,
                &so,
                &["-shared", "-fPIC", &format!("-O{opt}")],
            );
            // ...and the plain executable, i.e. what CMake builds.
            compile(&c_source, &exe, &[&format!("-O{opt}")]);
        }

        println!("cargo:rustc-env=C_DRIVER_SO_O{opt}={}", so.display());
        println!("cargo:rustc-env=C_DRIVER_BIN_O{opt}={}", exe.display());
    }
}

fn compile(c_source: &Path, output: &Path, flags: &[&str]) {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let mut cmd = Command::new(&cc);
    cmd.args(flags).arg("-o").arg(output).arg(c_source);

    match cmd.status() {
        Ok(status) if status.success() => {}
        Ok(status) => println!(
            "cargo:warning={cc} {flags:?} failed ({status}); {} will be missing",
            output.display()
        ),
        Err(e) => println!(
            "cargo:warning=cannot run {cc} ({e}); {} will be missing",
            output.display()
        ),
    }
}
