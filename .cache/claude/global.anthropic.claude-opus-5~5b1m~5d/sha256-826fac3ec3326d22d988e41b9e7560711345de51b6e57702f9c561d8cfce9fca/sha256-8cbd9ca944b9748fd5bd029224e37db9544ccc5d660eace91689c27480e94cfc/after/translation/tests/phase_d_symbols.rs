//! Phase D — symbol parity enforced as a test.
//!
//! Every dynamic symbol the C `.so` defines must also be defined by the Rust
//! `.so` under the exact same name.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn defined_dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Global text/data only; skip weak and Rust/libc runtime internals.
            if kind == "T" || kind == "D" || kind == "B" || kind == "R" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .filter(|n| {
            // Ignore linker/runtime-provided names that are not library API.
            !n.starts_with("_")
                && n != "rust_eh_personality"
                && !n.contains('.')
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

fn c_so() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    std::fs::read_dir(&dir)
        .expect("build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name().unwrap().to_string_lossy().starts_with("lib")
        })
        .expect("no lib*.so in c_src/build")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let candidates = [
        exe.parent().unwrap().parent().unwrap().join("libgotomach_lib.so"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libgotomach_lib.so"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libgotomach_lib.so"),
    ];
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .expect("run `cargo build` first")
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&c_so());
    let r = defined_dynamic_symbols(&rust_so());

    assert!(
        c.contains(&"gotomach".to_string()),
        "sanity: C .so must export gotomach, got {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );

    // The four documented API symbols specifically.
    for s in ["gotomach", "process_value", "double_value", "triple_value"] {
        assert!(c.contains(&s.to_string()), "C .so missing {s}");
        assert!(r.contains(&s.to_string()), "Rust .so missing {s}");
    }

    // The C `static` helpers must not leak into either dynamic symbol table.
    for s in [
        "is_valid_state",
        "check_char_flag",
        "init_processor",
        "cleanup_processor",
    ] {
        assert!(!c.contains(&s.to_string()), "C .so unexpectedly exports {s}");
        assert!(!r.contains(&s.to_string()), "Rust .so unexpectedly exports {s}");
    }
}

#[test]
fn all_symbols_resolve_via_dlsym() {
    // Loading through libloading already proves dlsym resolution works for the
    // full API surface in both shared objects.
    let p = common::pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
}
