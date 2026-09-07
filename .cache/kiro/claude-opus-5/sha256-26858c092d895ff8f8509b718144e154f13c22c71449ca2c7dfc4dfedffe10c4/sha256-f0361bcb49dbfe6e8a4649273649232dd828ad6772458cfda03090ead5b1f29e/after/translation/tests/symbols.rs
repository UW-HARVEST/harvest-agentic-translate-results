//! Phase D — symbol parity, enforced as a test so it can never drift.
//!
//! Runs `nm -D --defined-only` on both shared objects and asserts the Rust
//! `.so` exports every symbol the C `.so` does, with the exact same name.

mod harness;

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("c_src/build missing — build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().expect("no .so in c_src/build")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap();
    let p = dir.join("libcollided_lib.so");
    if p.exists() {
        return p;
    }
    for prof in ["debug", "release"] {
        let c = manifest_dir().join("target").join(prof).join("libcollided_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("libcollided_lib.so not found");
}

/// Defined (exported) dynamic symbols, excluding the platform/runtime noise
/// that a Rust cdylib always carries (`_init`, `_fini`, `rust_*`, …).
fn defined_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .filter(|s| {
            !s.starts_with('_')
                && !s.starts_with("rust_")
                && s != "__bss_start"
                && s != "_edata"
                && s != "_end"
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn symbol_parity_c_so_subset_of_rust_so() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());
    assert_eq!(c.len(), 10, "expected 10 exported C symbols, got {c:?}");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {missing:?}\n  C   : {c:?}\n  Rust: {r:?}",
        missing.len()
    );

    // Exact expected set, so an accidental rename is caught too.
    let expected = [
        "c2AABBtoAABB",
        "c2CircletoAABB",
        "c2CircletoCircle",
        "c2Clampv",
        "c2Dot",
        "c2Maxv",
        "c2Minv",
        "c2Sub",
        "c2V",
        "collided",
    ];
    assert_eq!(c, expected, "C export set changed unexpectedly");
    for e in expected {
        assert!(r.contains(&e.to_string()), "Rust .so does not export {e}");
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let so = rust_so();
    let out = Command::new("nm")
        .args(["-D", "-u", so.to_str().unwrap()])
        .output()
        .expect("nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| s.starts_with("c2") || *s == "collided")
        .collect();
    assert!(bad.is_empty(), "Rust .so imports library symbols it should define: {bad:?}");
}

/// The C project builds no executable, only `add_library(... SHARED src/lib.c)`,
/// so there is no binary stdout to compare. Assert that stays true.
#[test]
fn project_builds_no_executable_driver() {
    let cmake = manifest_dir().parent().unwrap().join("c_src/CMakeLists.txt");
    let text = std::fs::read_to_string(&cmake).expect("CMakeLists.txt");
    assert!(
        !text.contains("add_executable"),
        "c_src now builds an executable — Phase B must also diff its stdout"
    );
    assert!(text.contains("add_library"));
}
