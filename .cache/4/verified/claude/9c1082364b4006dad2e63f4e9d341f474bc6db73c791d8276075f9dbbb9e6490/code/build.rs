// Build script: compiles the reference C implementation (c_src/src/main.c)
// into shared libraries so the differential integration tests can dlopen both
// the C and the Rust `.so` and compare them across the FFI boundary.
//
// Nothing in c_src/ is modified; the objects land in c_src/build/ (the
// directory the project's own CMake build also writes into) and their paths
// are handed to the test code through `cargo:rustc-env`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let c_source = manifest_dir.join("c_src").join("src").join("main.c");
    let out_dir = manifest_dir.join("c_src").join("build");

    println!("cargo:rerun-if-changed={}", c_source.display());
    println!("cargo:rerun-if-changed=build.rs");

    std::fs::create_dir_all(&out_dir).expect("cannot create c_src/build");

    // Two optimisation levels: the default (what c_src/CMakeLists.txt produces
    // with no CMAKE_BUILD_TYPE) and -O2, so the tests can confirm the Rust
    // matches the C regardless of how aggressively gcc optimises it.
    let variants: [(&str, &str); 2] = [("libcref.so", "-O0"), ("libcref_o2.so", "-O2")];

    for (name, opt) in variants {
        let so = out_dir.join(name);
        build_so(&c_source, &so, opt);
        let env_key = if opt == "-O2" {
            "C_REF_SO_O2"
        } else {
            "C_REF_SO"
        };
        println!("cargo:rustc-env={}={}", env_key, so.display());
    }

    // A guaranteed-present copy of the CMake `driver` executable, built with
    // the same flags CMakeLists.txt uses, so the end-to-end executable
    // comparison works even if `cmake --build` has not been run.
    build_exe(&c_source, &out_dir.join("driver_ref"));

    println!("cargo:rustc-env=C_DRIVER_SRC={}", c_source.display());
}

fn build_exe(src: &Path, dst: &Path) {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&cc)
        .arg("-fPIE")
        .arg("-o")
        .arg(dst)
        .arg(src)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {cc}: {e}"));
    assert!(status.success(), "{cc} failed to build {}", dst.display());
}

fn build_so(src: &Path, dst: &Path, opt: &str) {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&cc)
        .arg("-shared")
        .arg("-fPIC")
        .arg(opt)
        .arg("-o")
        .arg(dst)
        .arg(src)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {cc}: {e}"));
    assert!(
        status.success(),
        "{cc} failed to build {} from {}",
        dst.display(),
        src.display()
    );
}
