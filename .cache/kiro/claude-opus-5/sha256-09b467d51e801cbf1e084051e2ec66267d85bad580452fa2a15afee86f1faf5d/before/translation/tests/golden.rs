//! Golden tests: every `(input, output)` pair below was captured from the
//! original C program in `c_src/` (built with the supplied CMakeLists, i.e.
//! `gcc` with no optimisation flags) and must be reproduced byte for byte.

use std::io::Write;
use std::process::{Command, Stdio};

const CASES: &[(&str, &str)] = &[
    ("0 0.5 0.25 0.125 0 0 0 0 0 0 0 0", "-0.142593384\n"),
    ("0 12.75 -3.5 8.125 16 16 16 0 0 0 0 0", "-0.516644061\n"),
    ("1 0.5 0.25 0.125 0 0 0 7 0 0 0 0", "-0.076374203\n"),
    ("1 -20.5 33.25 -0.75 8 4 2 255 0 0 0 0", "0.448242188\n"),
    ("2 0.5 0.5 0.5 0 0 0 0 2 0.5 1 6", "0.421875\n"),
    ("2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 8", "0.620870948\n"),
    ("3 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6", "-0.5\n"),
    ("3 -7.125 4.5 0.875 0 0 0 0 2.5 0.4 0 10", "0.09392827\n"),
    ("4 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6", "0.5\n"),
    ("4 9.25 -1.5 6.75 0 0 0 0 1.75 0.6 0 5", "0.87352705\n"),
    ("5 0.5 0.5 0.5 0 0 0 0 0 0 0 0", "0\n"),
    ("5 -12.25 7.5 -3.125 6 10 14 200 0 0 0 0", "0.402987331\n"),
    ("5 100.5 -100.5 50.25 3 5 7 9 0 0 0 0", "0.118530273\n"),
    // `which` outside 0..=5 returns NAN
    ("-1 1 2 3 0 0 0 0 0 0 0 0", "nan\n"),
    ("6 1 2 3 0 0 0 0 0 0 0 0", "nan\n"),
    // Failed/short scanf conversions leave the remaining values at zero.
    ("", "0\n"),
    ("3", "0\n"),
    ("0 1 2", "0\n"),
    ("abc", "0\n"),
    ("0 1e 2 3 0 0 0 5 2 .5 1 4", "0\n"),
    ("0 1e- 2 3 0 0 0 5 2 .5 1 4", "0\n"),
    ("0 .5 1. -.25 0 0 0 5 2 .5 1 4", "0.120605469\n"),
    // glibc's %f accepts hex floats, rejects partial "infinity", and ignores
    // the nan(...) form.
    ("0 0x10 0x1p-1 1 0 0 0 0 0 0 0 0", "0.25\n"),
    ("0 nan 0 0 0 0 0 0 0 0 0 0", "nan\n"),
    ("0 inf 1 1 0 0 0 0 0 0 0 0", "-nan\n"),
    ("0 -inf 1 1 0 0 0 0 0 0 0 0", "-nan\n"),
    ("0 infi 1 1 0 0 0 0 0 0 0 0", "0\n"),
    ("0 nan(x) 1 1 0 0 0 0 0 0 0 0", "nan\n"),
    // NaN sign as produced by the unoptimised C build.
    ("2 -1e+20 0.0773796436 14.8969174 1 4 2 2 -2 0 1 8", "-nan\n"),
    ("3 1e20 1 1 0 0 0 0 2 0 0 4", "-nan\n"),
    ("4 1e20 1 1 0 0 0 0 2 0 0 4", "nan\n"),
    // %d overflow truncation
    ("99999999999999999999 1 2 3 0 0 0 0 0 0 0 0", "nan\n"),
    ("2147483648 1 2 3 0 0 0 0 0 0 0 0", "nan\n"),
    // scanf reads across newlines
    ("0\n0.5\n0.25\n0.125\n0\n0\n0\n0\n0\n0\n0\n0\n", "-0.142593384\n"),
    ("  \t\n 1 \t 0.5 0.5 0.5 0 0 0 3 0 0 0 0", "-0.125\n"),
    ("0 -0.0 -0.0 -0.0 0 0 0 0 0 0 0 0", "0\n"),
    ("0 -2147483648 0 0 0 0 0 0 0 0 0 0", "0\n"),
    // out-of-range wraps for the non-pow2 variant read past the tables
    ("5 0.5 0.5 0.5 512 512 512 0 0 0 0 0", "0\n"),
    ("5 0.5 0.5 0.5 -4 -4 -4 0 0 0 0 0", "0\n"),
    ("3 0.001 0.002 0.003 1.0000001 0.9999999 1 20", "0\n"),
];

fn driver_path() -> std::path::PathBuf {
    // target/<profile>/deps/<test binary> -> target/<profile>/driver
    let mut p = std::env::current_exe().expect("test exe path");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("driver")
}

#[test]
fn golden_outputs_match_c() {
    let exe = driver_path();
    assert!(
        exe.exists(),
        "build the binary first (cargo build); missing {}",
        exe.display()
    );

    let mut failures = Vec::new();
    for (input, expected) in CASES {
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn driver");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input.as_bytes())
            .expect("write stdin");
        let out = child.wait_with_output().expect("run driver");
        let got = String::from_utf8_lossy(&out.stdout).to_string();
        if got != *expected {
            failures.push(format!("input {input:?}: expected {expected:?}, got {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
