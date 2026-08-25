use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest_dir.join("tests/support/fail_malloc.c");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("libfail_malloc.so");
    let rust_source = manifest_dir.join("src/lib.rs");
    let rust_output =
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("libarity_lib_under_test.so");

    let status = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(&source)
        .args(["-o"])
        .arg(&output)
        .args(["-ldl"])
        .status()
        .expect("run C compiler for allocation-failure test interposer");
    assert!(
        status.success(),
        "failed to build allocation-failure interposer"
    );

    let status = Command::new(env::var_os("RUSTC").unwrap())
        .args(["--crate-name", "arity_lib", "--crate-type", "cdylib"])
        .args(["--edition", "2024"])
        .arg(&rust_source)
        .args(["-o"])
        .arg(&rust_output)
        .status()
        .expect("run Rust compiler for differential-test shared library");
    assert!(
        status.success(),
        "failed to build differential-test shared library"
    );

    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", rust_source.display());
    println!("cargo:rustc-env=FAIL_MALLOC_SO={}", output.display());
    println!("cargo:rustc-env=RUST_TEST_SO={}", rust_output.display());
}
