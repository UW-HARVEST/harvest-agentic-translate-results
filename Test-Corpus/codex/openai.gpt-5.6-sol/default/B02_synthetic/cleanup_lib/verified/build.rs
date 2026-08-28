use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let source = PathBuf::from("tests/support/failure_interposer.c");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set"))
        .join("libfailure_interposer.so");

    println!("cargo:rerun-if-changed={}", source.display());

    let status = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .status()
        .expect("failed to execute cc for the test interposer");
    assert!(status.success(), "failed to build the test interposer");

    println!("cargo:rustc-env=FAILURE_INTERPOSER={}", output.display());
}
