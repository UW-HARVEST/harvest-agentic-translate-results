use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest_dir.join("tests/alloc_fail_shim.c");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("liballoc_fail_shim.so");

    println!("cargo:rerun-if-changed={}", source.display());
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&output)
        .arg(&source)
        .status()
        .expect("failed to invoke cc for allocation-failure shim");
    assert!(status.success(), "failed to build allocation-failure shim");
    println!("cargo:rustc-env=ALLOC_FAIL_SHIM={}", output.display());
}
