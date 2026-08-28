//! Phase D — symbol parity gate, enforced as a test rather than by eyeball.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and the Rust `.so` must not depend on anything
//! outside libc / the platform runtime.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {:?} {} failed: {}",
        args,
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Strip the glibc/libgcc version suffix: `malloc@GLIBC_2.2.5` -> `malloc`.
fn base(sym: &str) -> &str {
    sym.split('@').next().unwrap_or(sym)
}

fn paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let (c, r) = common::impls();
    (c.path.clone(), r.path.clone())
}

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let (cp, rp) = paths();
    let c_defined = nm(&cp, &["-D", "--defined-only"]);
    let r_defined = nm(&rp, &["-D", "--defined-only"]);

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C  ({}) exports: {:?}\n\
         Rust ({}) exports: {:?}",
        missing.len(),
        missing,
        cp.display(),
        c_defined,
        rp.display(),
        r_defined
    );

    // The five documented entry points must actually be there (guards against
    // an empty/failed nm parse silently passing the diff above).
    for want in [
        "create_block",
        "allocate_block",
        "free_block",
        "compute_hash",
        "betagamma",
    ] {
        assert!(c_defined.contains(want), "C .so lost {want}");
        assert!(r_defined.contains(want), "Rust .so lost {want}");
    }
    assert_eq!(c_defined.len(), 5, "C export set changed: {c_defined:?}");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_dependencies() {
    let (_, rp) = paths();
    let undefined = nm(&rp, &["-D", "--undefined-only"]);

    // Everything the Rust .so imports must be glibc, the libgcc unwinder, or a
    // weak optional hook -- i.e. part of the platform, resolvable at load time.
    let allowed_unversioned: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];

    let mut bad = Vec::new();
    for sym in &undefined {
        let has_platform_version = sym.contains("@GLIBC_") || sym.contains("@GCC_");
        let b = base(sym);
        let is_unwinder = b.starts_with("_Unwind_");
        if has_platform_version || is_unwinder || allowed_unversioned.contains(&b) {
            continue;
        }
        bad.push(sym.clone());
    }
    assert!(
        bad.is_empty(),
        "Rust .so has {} unresolved non-libc symbol(s): {:?}",
        bad.len(),
        bad
    );

    // And the .so must really load (dlopen already succeeded in common::impls,
    // which proves every import resolved at runtime).
    assert!(!undefined.is_empty(), "nm produced no imports at all");
}

#[test]
fn both_libraries_are_actually_loadable_and_distinct() {
    let (cp, rp) = paths();
    assert!(cp.is_file(), "C .so missing: {}", cp.display());
    assert!(rp.is_file(), "Rust .so missing: {}", rp.display());
    assert_ne!(cp, rp);
    // Different files, so different inodes.
    let cm = std::fs::metadata(&cp).unwrap();
    let rm = std::fs::metadata(&rp).unwrap();
    assert!(cm.len() > 0 && rm.len() > 0);
}
