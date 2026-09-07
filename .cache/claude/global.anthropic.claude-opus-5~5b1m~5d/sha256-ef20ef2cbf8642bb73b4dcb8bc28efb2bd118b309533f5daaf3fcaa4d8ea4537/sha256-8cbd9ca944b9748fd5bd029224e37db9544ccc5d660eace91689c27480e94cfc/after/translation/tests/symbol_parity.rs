//! Phase D — symbol parity enforced as a test, so it cannot silently regress.
//!
//! Compares `nm -D` on the C `.so` and the Rust `.so`: every symbol the C `.so`
//! exports the Rust `.so` must export under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            // "<addr> <type> <name>" for defined symbols.
            if f.len() >= 3 {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&common::c_so_path());
    let r = defined_dynamic_symbols(&common::rust_so_path());

    assert!(
        c.contains("driver"),
        "sanity: C .so must export `driver`, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );
}

/// No undefined symbol in the Rust `.so` may be an untranslated project symbol;
/// they must all resolve to libc / libgcc.
#[test]
fn rust_so_has_no_unresolved_project_symbols() {
    let so = common::rust_so_path();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    let orphans: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| {
            !s.contains("GLIBC")
                && !s.contains("GCC_")
                && !s.starts_with("_ITM_")
                && s != "__gmon_start__"
        })
        .collect();
    assert!(
        orphans.is_empty(),
        "Rust .so has undefined non-libc symbols (untranslated code?): {orphans:?}"
    );
}

/// The C project builds no executable, so there is no binary stdout to diff.
/// Pinned as a test so the claim in `CONFIGS.md` stays true.
#[test]
fn c_project_builds_no_executable() {
    let cmake = std::fs::read_to_string(
        common::c_so_path()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("CMakeLists.txt"),
    )
    .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "CMakeLists.txt now builds an executable — a binary stdout diff is required"
    );
}
