//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Toolchain-emitted weak symbols that appear in every shared object and are not
/// part of the library's API surface.
const TOOLCHAIN: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__gmon_start__",
    "__cxa_finalize",
];

fn strip_version(s: &str) -> String {
    s.split('@').next().unwrap_or(s).to_string()
}

#[test]
fn c_and_rust_export_identical_symbols() {
    let c = c_so_path();
    let r = rust_so_path();

    let c_defined: BTreeSet<String> = nm(&["-D", "--defined-only"], &c)
        .into_iter()
        .map(|s| strip_version(&s))
        .filter(|s| !TOOLCHAIN.contains(&s.as_str()))
        .collect();
    let r_defined: BTreeSet<String> = nm(&["-D", "--defined-only"], &r)
        .into_iter()
        .map(|s| strip_version(&s))
        .filter(|s| !TOOLCHAIN.contains(&s.as_str()))
        .collect();

    let expected: BTreeSet<String> = [
        "apply_bit_operations",
        "envy",
        "init_config_from_env",
        "parse_env_numeric",
        "perform_operation",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    assert_eq!(
        c_defined, expected,
        "the C library's exported surface changed; regenerate SYMBOLS.md"
    );

    let missing_in_rust: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing_in_rust.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing_in_rust:?}"
    );

    let extra_in_rust: Vec<&String> = r_defined.difference(&c_defined).collect();
    assert!(
        extra_in_rust.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra_in_rust:?}"
    );

    assert_eq!(c_defined, r_defined, "symbol diff must be empty");
}

/// Every symbol the Rust `.so` imports must resolve — i.e. it must come from
/// libc/libgcc, never be an untranslated piece of the C library.
#[test]
fn rust_has_no_unresolved_non_libc_imports() {
    let r = rust_so_path();

    // Anything the C library itself defines must NOT appear as undefined in the
    // Rust object (that would mean the Rust object depends on the C one).
    let c_defined: BTreeSet<String> = nm(&["-D", "--defined-only"], &c_so_path())
        .into_iter()
        .map(|s| strip_version(&s))
        .collect();

    let r_undef: BTreeSet<String> = nm(&["-D", "--undefined-only"], &r)
        .into_iter()
        .map(|s| strip_version(&s))
        .filter(|s| !TOOLCHAIN.contains(&s.as_str()))
        .collect();

    let bad: Vec<&String> = r_undef.intersection(&c_defined).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined references to library functions: {bad:?}"
    );

    // And the loader must be able to satisfy every import.
    let ldd = Command::new("ldd")
        .arg("-r")
        .arg(&r)
        .output()
        .expect("ldd not available");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&ldd.stdout),
        String::from_utf8_lossy(&ldd.stderr)
    );
    assert!(
        !text.contains("undefined symbol") && !text.contains("not found"),
        "unresolved imports in the Rust .so:\n{text}"
    );
}

/// `CONFIGS.md` claims the crate declares no cargo features; keep that honest.
#[test]
fn cargo_toml_declares_no_features() {
    let manifest = std::fs::read_to_string(common::crate_root().join("Cargo.toml")).unwrap();
    let has_features = manifest
        .lines()
        .any(|l| l.trim_start().starts_with("[features]"));
    assert!(
        !has_features,
        "Cargo.toml now has a [features] table — CONFIGS.md's feature-combination \
         section and check_all_feature_combos.sh must be updated to enumerate it"
    );
}
