//! Phase D — symbol parity enforced as a test: every dynamic symbol the C `.so`
//! exports must also be exported by the Rust `.so`, with the exact same name.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Global text/data/bss/rodata only; skip local (lowercase) symbols.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = nm_defined(common::c_so_path());

    assert!(
        c_syms.contains("call_predict"),
        "sanity: C .so should export call_predict, got {c_syms:?}"
    );

    for profile in ["debug", "release"] {
        let rp: PathBuf = common::rust_so_path(profile);
        let r_syms = nm_defined(&rp);
        let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
        assert!(
            missing.is_empty(),
            "Rust .so ({profile}) is missing C-exported symbols: {missing:?}"
        );
        eprintln!("symbol parity OK for profile {profile}: {} C symbols", c_syms.len());
    }
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    for profile in ["debug", "release"] {
        let so = common::rust_so_path(profile);
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", "--format=posix"])
            .arg(&so)
            .output()
            .expect("run nm");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        let suspicious: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|n| {
                // Anything that looks like it came from the translated library
                // itself (BTAC1C2_*, call_predict, get_predict_func) must not
                // be undefined.
                n.starts_with("BTAC1C2") || *n == "call_predict" || *n == "get_predict_func"
            })
            .collect();
        assert!(
            suspicious.is_empty(),
            "Rust .so ({profile}) has undefined project symbols: {suspicious:?}"
        );
    }
}

/// `lib.h` declares `get_predict_func` but the C source never defines it, so
/// neither library may export it. (A stubbed export would be a false positive.)
#[test]
fn get_predict_func_is_not_exported_by_either_library() {
    let c_syms = nm_defined(common::c_so_path());
    assert!(
        !c_syms.contains("get_predict_func"),
        "C .so unexpectedly exports get_predict_func"
    );
    for profile in ["debug", "release"] {
        let r_syms = nm_defined(&common::rust_so_path(profile));
        assert!(
            !r_syms.contains("get_predict_func"),
            "Rust .so ({profile}) must not export the undefined get_predict_func"
        );
    }
}
