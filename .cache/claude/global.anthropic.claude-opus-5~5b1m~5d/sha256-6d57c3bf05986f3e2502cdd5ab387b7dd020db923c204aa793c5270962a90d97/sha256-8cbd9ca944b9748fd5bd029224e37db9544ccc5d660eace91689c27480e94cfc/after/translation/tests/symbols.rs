//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every dynamic symbol the C library exports must be exported by the Rust
//! library under the exact same name, and the Rust library must not leave any
//! non-libc symbol undefined.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, extra: &[&str]) -> Vec<String> {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    for e in extra {
        cmd.arg(e);
    }
    cmd.arg(path);
    let out = cmd.output().expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm {path:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn defined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["--defined-only"]).into_iter().collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = find_c_so();
    let r = find_rust_so();
    let cs = defined(&c);
    let rs = defined(&r);

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so ({r:?}) is missing {} symbol(s) exported by the C .so ({c:?}): {missing:?}",
        missing.len()
    );

    // sanity: the 16 documented symbols really are in there
    for want in [
        "helxo",
        "strkey",
        "stbds_rand_seed",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_hmdel_key",
        "stbds_shmode_func",
        "stbds_stralloc",
        "stbds_strreset",
    ] {
        assert!(cs.contains(want), "C .so should export {want}");
        assert!(rs.contains(want), "Rust .so should export {want}");
    }

    // `stbds_unit_tests` is extern-declared but never defined in the C TU, so
    // neither library may export it.
    assert!(!cs.contains("stbds_unit_tests"));
    assert!(!rs.contains("stbds_unit_tests"));
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let r = find_rust_so();
    let undef: Vec<String> = nm(&r, &["--undefined-only"])
        .into_iter()
        .filter(|s| !s.is_empty() && s != "U")
        .collect();
    // Everything the Rust cdylib imports must come from libc / libgcc's
    // unwinder / the dynamic loader.  Versioned imports (`name@GLIBC_x.y`,
    // `name@GCC_x.y`) are by construction resolved from the system libraries;
    // the unversioned remainder is the small, well-known loader set.
    let loader: BTreeSet<&str> = [
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__gmon_start__",
    ]
    .into_iter()
    .collect();
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| !s.contains('@') && !loader.contains(s.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unexpected:?}"
    );

    // Crucially: no symbol belonging to *this* library may be undefined —
    // i.e. nothing was left as a forward declaration / untranslated stub.
    let project: Vec<&String> = undef
        .iter()
        .filter(|s| s.starts_with("stbds_") || *s == "helxo" || *s == "strkey")
        .collect();
    assert!(
        project.is_empty(),
        "Rust .so leaves library symbols undefined (untranslated?): {project:?}"
    );
}

#[test]
fn phase_d_all_symbols_are_callable_through_dlopen() {
    // Loading the pair already resolves all 16 symbols in both libraries; if
    // any were absent, `Lib::open` would panic.
    let (p, _g) = libs();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.r.name, "Rust");
}
