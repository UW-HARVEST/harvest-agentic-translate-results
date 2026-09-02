//! Phase D — symbol parity, enforced as a test so it cannot drift.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`,
//! with the exact same name, and the Rust `.so` must have no unresolved
//! (non-libc) imports.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm {} {} failed: {}",
        extra,
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            // "<addr> <type> <name>" or "         <type> <name>"
            let name = l.split_whitespace().last()?;
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

fn rust_sos() -> Vec<(&'static str, PathBuf)> {
    let v = common::rust_so_paths();
    assert!(!v.is_empty(), "no Rust .so built");
    v
}

#[test]
fn c_defined_symbols_are_all_exported_by_rust() {
    let c = nm(&common::c_so_path(), "--defined-only");
    assert!(
        c.contains("searchAndReplace"),
        "the C .so does not export searchAndReplace: {c:?}"
    );
    for (name, path) in rust_sos() {
        let r = nm(&path, "--defined-only");
        let missing: Vec<&String> = c.difference(&r).collect();
        assert!(
            missing.is_empty(),
            "{name} ({}) is missing {} symbol(s) exported by the C .so: {missing:?}",
            path.display(),
            missing.len()
        );
        eprintln!(
            "{name}: {} C symbol(s) all present ({} defined dynamic symbols total)",
            c.len(),
            r.len()
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_imports() {
    for (name, path) in rust_sos() {
        let out = Command::new("ldd").arg("-r").arg(&path).output().expect("run ldd -r");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(bad.is_empty(), "{name} has unresolved imports: {bad:?}");
    }
}

#[test]
fn c_undefined_symbols_are_libc_only() {
    // Documents the import surface recorded in SYMBOLS.md: the C library imports
    // nothing but libc, so the Rust translation must not need a third-party
    // runtime either (only libc + libgcc_s are allowed).
    for (name, path) in rust_sos() {
        let out = Command::new("ldd").arg(&path).output().expect("run ldd");
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with("linux-vdso") || l.starts_with('/') {
                continue;
            }
            let lib = l.split_whitespace().next().unwrap_or("");
            assert!(
                lib.starts_with("libc.")
                    || lib.starts_with("libgcc_s.")
                    || lib.starts_with("libm.")
                    || lib.starts_with("ld-linux"),
                "{name} links unexpected library {lib:?}"
            );
        }
    }
}
