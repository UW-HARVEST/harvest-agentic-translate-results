//! Phase D — symbol parity enforced from inside the test suite.
//!
//! The shell script `check_all.sh` also does this, but encoding it as a test
//! means a regression (a dropped `#[no_mangle]`, a renamed export) fails
//! `cargo test` rather than only a manual step.

mod common;

use std::path::PathBuf;
use std::process::Command;

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`
/// under the exact same name.
#[test]
fn symbol_parity_with_c_so() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = std::env::var("C_SO").map(PathBuf::from).unwrap_or_else(|_| {
        root.parent().unwrap().join("c_src/build/libdriver.so")
    });
    let rust_so = std::env::var("RUST_SO").map(PathBuf::from).unwrap_or_else(|_| {
        let rel = root.join("target/release/libdriver.so");
        if rel.exists() { rel } else { root.join("target/debug/libdriver.so") }
    });

    let defined = |p: &PathBuf| -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(p)
            .output()
            .expect("nm is available");
        assert!(out.status.success(), "nm failed on {}", p.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(str::to_string))
            // Ignore the toolchain's own bookkeeping symbols, which are not part
            // of either library's API surface.
            .filter(|s| {
                !s.starts_with("_ITM_")
                    && !s.starts_with("__gmon")
                    && !s.starts_with("_fini")
                    && !s.starts_with("_init")
                    && !s.starts_with("__bss")
                    && !s.starts_with("_edata")
                    && !s.starts_with("_end")
            })
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let c_syms = defined(&c_so);
    let rust_syms = defined(&rust_so);
    assert!(!c_syms.is_empty(), "the C .so must export something");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing {} symbol(s) exported by the C .so: {missing:?}",
        rust_so.display(),
        missing.len()
    );

    // The three documented entry points must be present by name, so a future
    // change cannot satisfy the diff by shrinking the C side.
    for expected in ["driver", "forward_goto_example", "open_with_cleanup"] {
        assert!(
            c_syms.iter().any(|s| s == expected),
            "C .so should export {expected}"
        );
        assert!(
            rust_syms.iter().any(|s| s == expected),
            "Rust .so should export {expected}"
        );
    }
}

/// The Rust `.so` must have no unresolved non-libc dependencies — i.e. it must
/// actually load, which is already proven by the other suites, and `ldd` must
/// report nothing missing.
#[test]
fn no_unresolved_dependencies() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust_so = std::env::var("RUST_SO").map(PathBuf::from).unwrap_or_else(|_| {
        let rel = root.join("target/release/libdriver.so");
        if rel.exists() { rel } else { root.join("target/debug/libdriver.so") }
    });

    let out = Command::new("ldd").arg(&rust_so).output().expect("ldd available");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "unresolved shared-library dependency in {}:\n{text}",
        rust_so.display()
    );

    // Dlopening it is the real proof that every symbol resolves.
    let _ = common::rust_api();
}
