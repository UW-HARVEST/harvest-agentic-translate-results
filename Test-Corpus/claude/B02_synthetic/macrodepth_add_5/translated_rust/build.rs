// Build script: compile the C source as a shared library matching the
// currently selected Cargo features (OP, REPEAT). The path to the resulting
// .so is exposed to integration tests via the `C_LIB_PATH` env var.
//
// This only runs the C compilation when the cdylib tests need it; we always
// run it because the active feature set may change.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Pick OP from features.
    let op = if env::var_os("CARGO_FEATURE_SUB").is_some() {
        "sub"
    } else if env::var_os("CARGO_FEATURE_MUL").is_some() {
        "mul"
    } else {
        // add is the default + fallback
        "add"
    };

    // Pick REPEAT (lowest-numbered wins, matching lib.rs cfg ordering).
    let repeat: i32 = if env::var_os("CARGO_FEATURE_REPEAT_0").is_some() {
        0
    } else if env::var_os("CARGO_FEATURE_REPEAT_1").is_some() {
        1
    } else if env::var_os("CARGO_FEATURE_REPEAT_2").is_some() {
        2
    } else if env::var_os("CARGO_FEATURE_REPEAT_3").is_some() {
        3
    } else if env::var_os("CARGO_FEATURE_REPEAT_4").is_some() {
        4
    } else if env::var_os("CARGO_FEATURE_REPEAT_5").is_some() {
        5
    } else if env::var_os("CARGO_FEATURE_REPEAT_6").is_some() {
        6
    } else if env::var_os("CARGO_FEATURE_REPEAT_7").is_some() {
        7
    } else {
        5
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let c_src = manifest_dir.join("c_src").join("src").join("mdcore.c");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let so_path = out_dir.join(format!("libmdcore_{}_{}.so", op, repeat));

    // Compile only when missing or stale.
    let need_build = match (so_path.metadata(), c_src.metadata()) {
        (Ok(so), Ok(src)) => {
            so.modified().ok() < src.modified().ok()
        }
        _ => true,
    };

    if need_build {
        let status = Command::new("gcc")
            .arg("-fPIC")
            .arg("-shared")
            .arg(format!("-DOP={}", op))
            .arg(format!("-DREPEAT={}", repeat))
            .arg("-o")
            .arg(&so_path)
            .arg(&c_src)
            .status()
            .expect("gcc invocation failed");
        assert!(status.success(), "gcc returned non-zero");
    }

    println!("cargo:rustc-env=C_LIB_PATH={}", so_path.display());
    println!("cargo:rustc-env=DRIVER_OP={}", op);
    println!("cargo:rustc-env=DRIVER_REPEAT={}", repeat);
    println!("cargo:rerun-if-changed=c_src/src/mdcore.c");
    println!("cargo:rerun-if-changed=c_src/src/mdmacros.h");
    println!("cargo:rerun-if-changed=build.rs");
}
