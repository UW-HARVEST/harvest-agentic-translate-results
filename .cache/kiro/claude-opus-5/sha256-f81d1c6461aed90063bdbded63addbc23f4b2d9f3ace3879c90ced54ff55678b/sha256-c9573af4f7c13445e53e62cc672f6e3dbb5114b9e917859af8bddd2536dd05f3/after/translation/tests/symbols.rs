//! Phase D — symbol parity between the two shared objects, enforced as a test
//! so it cannot silently regress.

mod common;

use common::*;
use std::process::Command;

fn defined_syms(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.starts_with("_ITM_") && s != "__gmon_start__" && s != "__cxa_finalize")
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_syms(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name. The diff must be empty.
#[test]
fn symbol_parity_c_to_rust() {
    let c = defined_syms(&c_so_path());
    let r = defined_syms(&rust_so_path());
    assert!(
        c.contains(&"sieve".to_string()),
        "C .so does not export `sieve`; nm output: {c:?}"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
}

/// The Rust `.so` must not depend on any library beyond the ones already
/// present in the C `.so`'s dependency closure: glibc and libgcc. Every
/// undefined symbol must therefore carry a `@GLIBC_*` / `@GCC_*` version tag
/// (or be one of the unversioned ELF placeholders every shared object has).
///
/// The Rust standard library brings in `_Unwind_*` (libgcc) and the `mmap`
/// / `dl_iterate_phdr` / `pthread_key_*` family for its panic-backtrace and
/// TLS machinery; those are fine, they resolve against libraries the process
/// already has. What would *not* be fine is an import from a third-party
/// `.so` that a consumer of the C library would not have loaded.
#[test]
fn rust_so_resolves_only_against_libc_and_libgcc() {
    let unversioned_ok = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    let undef = undefined_syms(&rust_so_path());
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !(s.contains("@GLIBC_") || s.contains("@GCC_") || unversioned_ok.contains(&s.as_str()))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so imports symbols from outside glibc/libgcc: {bad:?}\n\
         all undefined: {undef:?}"
    );
    assert!(
        undef.iter().any(|s| s.starts_with("printf")),
        "Rust .so does not import libc printf — it is not using the shared \
         stdout FILE the C uses; undefined: {undef:?}"
    );

    // Confirm the same, empirically: the needed-library list must be a subset
    // of what the C .so needs, plus libgcc.
    let out = Command::new("objdump")
        .args(["-p", rust_so_path().to_str().unwrap()])
        .output();
    if let Ok(out) = out {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().filter(|l| l.trim_start().starts_with("NEEDED")) {
            let needed = line.split_whitespace().last().unwrap_or("");
            assert!(
                needed.starts_with("libc.so")
                    || needed.starts_with("libgcc_s.so")
                    || needed.starts_with("libm.so")
                    || needed.starts_with("ld-linux"),
                "Rust .so NEEDED an unexpected library: {needed}"
            );
        }
    }
}

/// The crate declares no cargo features, so there is exactly one configuration
/// to verify. This test fails if a feature is ever added without extending the
/// verification matrix.
#[test]
fn no_unverified_feature_configurations() {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml now declares features; Phases B and C must be re-run for \
         every combination and SYMBOLS.md updated"
    );
    assert!(
        !manifest.contains("cfg(feature"),
        "feature gating appeared in the manifest"
    );
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read lib.rs");
    assert!(
        !src.contains("feature = \""),
        "src/lib.rs contains feature gating but Cargo.toml declares no features"
    );
}
