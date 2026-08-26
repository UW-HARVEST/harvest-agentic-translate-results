//! Build helper for the differential test-suite.
//!
//! It compiles the *unmodified* C sources in `c_src/` twice:
//!
//! * `libcdriver.so` - a shared object exporting `process_buffer`, used by the
//!   `libloading` based differential tests, and
//! * `c_driver`      - the original command line program (`main.c` + `lib.c`),
//!   used by the CLI level differential test.
//!
//! Nothing inside `c_src/` is ever written to; all artefacts land in `OUT_DIR`.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let lib_c = manifest.join("c_src/src/lib.c");
    let main_c = manifest.join("c_src/src/main.c");

    println!("cargo:rerun-if-changed={}", lib_c.display());
    println!("cargo:rerun-if-changed={}", main_c.display());
    println!("cargo:rerun-if-changed=build.rs");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    // --- shared object -----------------------------------------------------
    let so = out.join("libcdriver.so");
    run(
        Command::new(&cc)
            .args(["-O2", "-shared", "-fPIC", "-o"])
            .arg(&so)
            .arg(&lib_c),
    );

    // --- reference executable ---------------------------------------------
    let exe = out.join("c_driver");
    run(Command::new(&cc)
        .args(["-O2", "-o"])
        .arg(&exe)
        .arg(&main_c)
        .arg(&lib_c));

    println!("cargo:rustc-env=C_SO_PATH={}", so.display());
    println!("cargo:rustc-env=C_DRIVER_PATH={}", exe.display());
}

fn run(cmd: &mut Command) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {cmd:?}: {e}"));
    assert!(status.success(), "command failed: {cmd:?} -> {status}");
}
