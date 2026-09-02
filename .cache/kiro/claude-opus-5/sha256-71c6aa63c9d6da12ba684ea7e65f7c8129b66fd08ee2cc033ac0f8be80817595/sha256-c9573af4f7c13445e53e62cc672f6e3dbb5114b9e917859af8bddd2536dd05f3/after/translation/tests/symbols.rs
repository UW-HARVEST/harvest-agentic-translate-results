// Phase A / Phase D — dynamic-symbol parity between the C `.so` and the Rust `.so`.
//
// This re-derives the `nm -D` diff at test time so that the SYMBOLS.md gate cannot
// silently rot when either side changes.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    assert!(out.status.success(), "nm failed on {so:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

fn nm_undefined(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm").args(["-D", "-u"]).arg(so).output().expect("failed to run `nm`");
    assert!(out.status.success(), "nm -u failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

#[test]
fn symbol_parity_c_vs_rust() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());

    // Sanity: the C side must actually contain the surface we think it does.
    for s in EXPECTED_SYMBOLS {
        assert!(c.contains(s), "C .so unexpectedly does not export `{s}`");
    }
    assert_eq!(
        c.len(),
        EXPECTED_SYMBOLS.len(),
        "C .so exports symbols not covered by SYMBOLS.md: {:?}",
        c.iter().filter(|s| !EXPECTED_SYMBOLS.contains(&s.as_str())).collect::<Vec<_>>()
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );
}

/// Every symbol must be dlsym-able from the Rust `.so`, not merely present in `nm`.
#[test]
fn every_symbol_is_dlsym_able_from_both() {
    // Api::open panics with the offending symbol name if any lookup fails.
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}

/// The Rust `.so` must not import anything beyond libc / the Rust runtime.
#[test]
fn rust_imports_only_libc_and_runtime() {
    let u = nm_undefined(&rust_so_path());
    let allowed_prefixes = ["_Unwind_", "__", "_ITM_"];
    let leftover: Vec<&String> = u
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        // everything else must be a plain libc function name (lowercase, no '::')
        .filter(|s| s.contains("::") || s.contains('$'))
        .collect();
    assert!(leftover.is_empty(), "Rust .so has non-libc undefined symbols: {leftover:?}");
}

/// Phase D: the feature matrix. `Cargo.toml` declares no `[features]` table, so
/// there is exactly one build configuration. If a feature is ever added, this
/// fails, forcing Phases B and C to be re-run per combination.
#[test]
fn no_cargo_features_declared() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let has_features = manifest.lines().any(|l| l.trim() == "[features]");
    assert!(
        !has_features,
        "Cargo.toml now declares [features]; Phases B and C must be re-run for \
         every feature combination (see scripts/check_features.sh)"
    );
    assert!(
        !manifest.contains("cfg(feature"),
        "feature-conditional code appeared; re-run the Phase D matrix"
    );
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    assert!(!lib.contains("feature = \""), "src/lib.rs became feature-conditional");
}
