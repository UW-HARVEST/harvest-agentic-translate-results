use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/config.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/mdcore.rs");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("libmd_driver_test.so");
    let rustc = env::var_os("RUSTC").unwrap();

    let mut command = Command::new(rustc);
    command
        .current_dir(&manifest_dir)
        .arg("--crate-name")
        .arg("md_driver_test")
        .arg("--crate-type")
        .arg("cdylib")
        .arg("--edition")
        .arg("2024")
        .arg("src/lib.rs")
        .arg("-o")
        .arg(&output);

    for feature in ["add", "sub", "mul", "0", "1", "2", "3", "4", "5", "6", "7"] {
        let environment_name = format!("CARGO_FEATURE_{}", feature.to_uppercase());
        if env::var_os(environment_name).is_some() {
            command.arg("--cfg").arg(format!("feature=\"{feature}\""));
        }
    }

    let result = command.output().expect("run rustc for differential cdylib");
    if !result.status.success() {
        panic!(
            "failed to build differential cdylib\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    println!("cargo:rustc-env=MD_RUST_DYLIB={}", output.display());
}
