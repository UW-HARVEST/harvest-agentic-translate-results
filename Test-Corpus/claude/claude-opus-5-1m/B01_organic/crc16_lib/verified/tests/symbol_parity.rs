//! Phase D — automated symbol-parity check.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name. The diff must be empty.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn exported_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run `nm` (binutils required for the symbol-parity test)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep global text/data/bss/weak definitions; drop everything else.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols the Rust std runtime unavoidably exports from a cdylib. These are
/// *additions*, never substitutes for C symbols, so they are allowed on the
/// Rust side only.
fn is_rust_runtime_symbol(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("_R")
        || s.starts_with("rust_")
        || s.starts_with("__rust_")
        || s.starts_with("_ITM_")
        || s.starts_with("__cxa")
        || s.starts_with("_Unwind")
        || s == "__gmon_start__"
        || s == "_init"
        || s == "_fini"
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let l = common::libs();
    let c_syms = exported_symbols(&l.c_path);
    let rust_syms = exported_symbols(&l.rust_path);

    println!("C   .so: {} ({} symbols)", l.c_path.display(), c_syms.len());
    for s in &c_syms {
        println!("  C  {s}");
    }
    println!(
        "Rust .so: {} ({} symbols)",
        l.rust_path.display(),
        rust_syms.len()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         Add the #[no_mangle] export, or translate the missing C source.",
        missing.len()
    );

    // The C exports at least `crc16`; guard against a degenerate empty diff.
    assert!(
        c_syms.contains("crc16"),
        "sanity check failed: C .so does not export `crc16`; symbol extraction is broken"
    );
}

#[test]
fn rust_exports_no_unexpected_public_api() {
    let l = common::libs();
    let c_syms = exported_symbols(&l.c_path);
    let rust_syms = exported_symbols(&l.rust_path);

    let extra: Vec<&String> = rust_syms
        .difference(&c_syms)
        .filter(|s| !is_rust_runtime_symbol(s))
        .collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports non-runtime symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn static_table_is_not_exported_by_either_library() {
    let l = common::libs();
    // `tflac_crc16_tables` is `static const` in lib.h -> internal linkage.
    // The Rust translation must keep it private too.
    for (label, path) in [("C", &l.c_path), ("Rust", &l.rust_path)] {
        let syms = exported_symbols(path);
        assert!(
            !syms.iter().any(|s| s.contains("crc16_tables")
                || s.contains("CRC16_TABLES")
                || s.contains("TFLAC")),
            "{label} .so unexpectedly exports the internal table: {syms:?}"
        );
    }
}

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let l = common::libs();
    println!("C   .so = {}", l.c_path.display());
    println!("Rust .so = {}", l.rust_path.display());
    assert_ne!(
        l.c_path.canonicalize().unwrap(),
        l.rust_path.canonicalize().unwrap(),
        "harness is comparing a library against itself!"
    );
    // Skip the default-layout checks when the paths were overridden via env.
    if std::env::var_os("CRC16_C_SO").is_none() {
        assert!(l.c_path.to_string_lossy().contains("c_src"));
    }
    if std::env::var_os("CRC16_RUST_SO").is_none() {
        assert!(l.rust_path.to_string_lossy().contains("libcrc16_lib"));
    }
}
