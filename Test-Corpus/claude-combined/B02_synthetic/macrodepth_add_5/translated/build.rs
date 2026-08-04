// Build the C source as a shared library so FFI tests can compare both
// implementations through libloading. The C source under c_src/ is NEVER
// modified — we only invoke gcc with the same -DOP / -DREPEAT macros that
// the active Cargo features represent.

use std::path::PathBuf;
use std::process::Command;

fn pick_op() -> &'static str {
    let add = std::env::var_os("CARGO_FEATURE_ADD").is_some();
    let sub = std::env::var_os("CARGO_FEATURE_SUB").is_some();
    let mul = std::env::var_os("CARGO_FEATURE_MUL").is_some();
    let count = [add, sub, mul].iter().filter(|x| **x).count();
    if count != 1 {
        panic!("exactly one of features add/sub/mul must be enabled (got {})", count);
    }
    if add { "add" } else if sub { "sub" } else { "mul" }
}

fn pick_repeat() -> &'static str {
    let names = ["0", "1", "2", "3", "4", "5", "6", "7"];
    let mut chosen: Option<&'static str> = None;
    for n in &names {
        let env = format!("CARGO_FEATURE_{}", n);
        if std::env::var_os(&env).is_some() {
            if chosen.is_some() {
                panic!("multiple REPEAT features enabled");
            }
            chosen = Some(n);
        }
    }
    chosen.expect("exactly one of features 0..=7 must be enabled")
}

fn main() {
    println!("cargo:rerun-if-changed=c_src/src/mdcore.c");
    println!("cargo:rerun-if-changed=c_src/src/mdmain.c");
    println!("cargo:rerun-if-changed=c_src/src/mdmacros.h");
    println!("cargo:rerun-if-changed=build.rs");
    // Re-run when feature env vars change (cargo sets CARGO_FEATURE_<NAME>=1).
    for feat in ["ADD", "SUB", "MUL", "0", "1", "2", "3", "4", "5", "6", "7"] {
        println!("cargo:rerun-if-env-changed=CARGO_FEATURE_{}", feat);
    }

    let op = pick_op();
    let repeat = pick_repeat();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest_dir.join("target").join("c_so");
    std::fs::create_dir_all(&out_dir).expect("create target/c_so");
    // Always overwrite to reflect the active OP/REPEAT features.
    let so_path = out_dir.join("libdriver_c.so");

    let src = manifest_dir.join("c_src").join("src").join("mdcore.c");

    let status = Command::new("gcc")
        .args([
            "-shared",
            "-fPIC",
            "-O2",
            "-Wno-implicit-function-declaration",
        ])
        .arg(format!("-DOP={}", op))
        .arg(format!("-DREPEAT={}", repeat))
        .arg(&src)
        .arg("-o")
        .arg(&so_path)
        .status()
        .expect("failed to spawn gcc");
    if !status.success() {
        panic!("gcc failed building C shared library");
    }
    // Touch the .so so its mtime reflects this configuration; helps when
    // diagnosing caching issues across feature combos.

    println!("cargo:rustc-env=DRIVER_C_SO={}", so_path.display());
    println!("cargo:rustc-env=DRIVER_OP={}", op);
    println!("cargo:rustc-env=DRIVER_REPEAT={}", repeat);
}
