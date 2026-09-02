// Phase C — ERRORS.md row 18: the `checkshift` malloc-failure branch.
//
// `malloc(sizeof(ComputeState))` is the library's only allocation, and its
// failure branch (`return -1`) is unreachable without help. This test builds an
// LD_PRELOAD interposer that fails malloc(12) inside a one-call window, plus a
// small C driver that dlopens BOTH .so files and calls their exported
// `checkshift`, and asserts C and Rust return the same sentinel and print the
// same bytes.
//
// The driver is a C program rather than this test process so that no Rust runtime
// allocation can land inside the failure window.

use std::path::{Path, PathBuf};
use std::process::Command;

fn aux_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/aux")
}

fn out_dir() -> PathBuf {
    let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/malloc_fail");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn cc() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

fn compile(args: &[&str], what: &str) {
    let out = Command::new(cc())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {} for {what}: {e}", cc()));
    assert!(
        out.status.success(),
        "compiling {what} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {dir:?}: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().unwrap_or_else(|| panic!("no .so in {dir:?}"))
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libcheckshift_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no libcheckshift_lib.so under {base:?}");
}

#[test]
fn err18_checkshift_malloc_failure() {
    let aux = aux_dir();
    let out = out_dir();
    let shim = out.join("libmalloc_shim.so");
    let driver = out.join("malloc_fail_driver");

    compile(
        &[
            "-shared",
            "-fPIC",
            "-O1",
            "-o",
            shim.to_str().unwrap(),
            aux.join("malloc_shim.c").to_str().unwrap(),
            "-ldl",
        ],
        "malloc_shim.c",
    );
    compile(
        &[
            "-O1",
            "-o",
            driver.to_str().unwrap(),
            aux.join("malloc_fail_driver.c").to_str().unwrap(),
            "-ldl",
        ],
        "malloc_fail_driver.c",
    );

    let output = Command::new(&driver)
        .arg(c_so_path())
        .arg(rust_so_path())
        .env("LD_PRELOAD", &shim)
        .output()
        .expect("failed to run malloc_fail_driver");

    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stderr}");
    match output.status.code() {
        Some(0) => {}
        Some(1) => panic!("ERRORS.md row 18: C and Rust DIVERGE on the malloc-failure path:\n{stderr}"),
        other => panic!("malloc-failure harness problem (exit {other:?}):\n{stderr}"),
    }
    assert!(stderr.contains("row 18 ok"), "driver did not confirm the row:\n{stderr}");
}

/// Regression guard for the divergence this file uncovered: the Rust `.so` must
/// perform the same allocator calls as the C `.so`. LLVM recognises
/// `malloc`/`free` by name and had promoted the 12-byte `ComputeState` block to
/// registers, deleting the allocation and with it the whole failure branch.
#[test]
fn err18b_allocator_call_parity() {
    let aux = aux_dir();
    let out = out_dir();
    let shim = out.join("libmalloc_shim_parity.so");
    let driver = out.join("alloc_parity_driver");

    compile(
        &[
            "-shared",
            "-fPIC",
            "-O1",
            "-o",
            shim.to_str().unwrap(),
            aux.join("malloc_shim.c").to_str().unwrap(),
            "-ldl",
        ],
        "malloc_shim.c",
    );
    compile(
        &[
            "-O1",
            "-o",
            driver.to_str().unwrap(),
            aux.join("alloc_parity_driver.c").to_str().unwrap(),
            "-ldl",
        ],
        "alloc_parity_driver.c",
    );

    let output = Command::new(&driver)
        .arg(c_so_path())
        .arg(rust_so_path())
        .env("LD_PRELOAD", &shim)
        .output()
        .expect("failed to run alloc_parity_driver");

    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stderr}");
    match output.status.code() {
        Some(0) => {}
        Some(1) => panic!("Rust and C allocator call counts DIVERGE:\n{stderr}"),
        other => panic!("allocator-parity harness problem (exit {other:?}):\n{stderr}"),
    }
    assert!(stderr.contains("allocator parity ok"), "driver did not confirm parity:\n{stderr}");
}
