// End-to-end test: build the C executable from c_src/ separately and run
// both binaries with identical CLI args; their stdout must be byte-identical.

use std::path::PathBuf;
use std::process::Command;

fn rust_bin() -> PathBuf {
    // env!("CARGO_BIN_EXE_driver") is the path to the rust binary built by
    // cargo for tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn c_bin() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let op = env!("DRIVER_OP");
    let repeat = env!("DRIVER_REPEAT");
    // Include OP/REPEAT in the path so different feature combos do not
    // shadow each other when running tests across configurations.
    let bin = manifest_dir
        .join("target")
        .join("c_bin")
        .join(format!("driver_c_{}_{}", op, repeat));
    if !bin.exists() {
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        let core = manifest_dir.join("c_src").join("src").join("mdcore.c");
        let main = manifest_dir.join("c_src").join("src").join("mdmain.c");
        let status = Command::new("gcc")
            .args(["-O2", "-Wno-implicit-function-declaration"])
            .arg(format!("-DOP={}", op))
            .arg(format!("-DREPEAT={}", repeat))
            .arg(&core)
            .arg(&main)
            .arg("-o")
            .arg(&bin)
            .status()
            .expect("spawn gcc");
        assert!(status.success(), "gcc failed");
    }
    bin
}

fn run(bin: &PathBuf, args: &[&str]) -> (Vec<u8>, Vec<u8>, i32) {
    let out = Command::new(bin)
        .args(args)
        .output()
        .expect("run binary");
    (out.stdout, out.stderr, out.status.code().unwrap_or(-1))
}

#[test]
fn binary_outputs_match_for_various_inputs() {
    let cb = c_bin();
    let rb = rust_bin();
    let inputs: &[(&str, &str)] = &[
        ("0", "0"),
        ("3", "4"),
        ("10", "-5"),
        ("-3", "7"),
        ("100", "50"),
        ("-2", "-2"),
        ("7", "6"),
        ("1", "1"),
    ];
    for (a, b) in inputs {
        let (co, _ce, _cs) = run(&cb, &[a, b]);
        let (ro, _re, _rs) = run(&rb, &[a, b]);
        assert_eq!(
            co, ro,
            "stdout differs for inputs ({}, {}):\nC:    {}\nRust: {}",
            a,
            b,
            String::from_utf8_lossy(&co),
            String::from_utf8_lossy(&ro)
        );
    }
}

#[test]
fn binary_usage_message_on_too_few_args() {
    let cb = c_bin();
    let rb = rust_bin();
    // Both should exit nonzero with "usage: ..." on stderr.
    let (_co, _ce, cs) = run(&cb, &[]);
    let (_ro, _re, rs) = run(&rb, &[]);
    assert_ne!(cs, 0, "C binary should exit nonzero");
    assert_ne!(rs, 0, "Rust binary should exit nonzero");
    assert_eq!(cs, rs, "Exit codes differ: C={} Rust={}", cs, rs);
}
