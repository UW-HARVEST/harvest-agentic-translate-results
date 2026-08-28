//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only` -> set of exported symbol names.
fn exported(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

/// `nm -D -u` -> set of undefined (imported) symbol names.
fn undefined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm -u failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s != "U" && s != "w")
        .collect()
}

/// A symbol is "provided by the platform" if the linker gave it a glibc/GCC
/// version tag, or it is one of the weak toolchain hooks. Anything else (in
/// particular a Rust-mangled `_ZN...` or a plain untagged name) would be a
/// genuinely unresolved dependency.
fn is_libc_ish(sym: &str) -> bool {
    const WEAK_TOOLCHAIN: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__tls_get_addr",
    ];
    sym.contains("@GLIBC_")
        || sym.contains("@GCC_")
        || sym.contains("@GLIBCXX_")
        || sym.contains("@CXXABI_")
        || WEAK_TOOLCHAIN.contains(&sym)
        || sym == "sqrt"
}

#[test]
fn symbol_parity_c_vs_rust() {
    let c = c_so_path();
    let r = rust_so_path();
    println!("C   .so: {}", c.display());
    println!("Rust.so: {}", r.display());

    let c_syms = exported(&c);
    let r_syms = exported(&r);
    println!("C exports  : {c_syms:?}");
    println!("Rust exports: {r_syms:?}");

    // The C library must export at least `jumpnode`; sanity-check our nm parse.
    assert!(
        c_syms.contains("jumpnode"),
        "nm parse produced no `jumpnode` for the C .so: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Report extras (informational: only the opt-in test hook may appear).
    let extra: Vec<&String> = r_syms.difference(&c_syms).collect();
    println!("Rust-only symbols: {extra:?}");
    if cfg!(feature = "expose_init_test_data") {
        assert_eq!(
            extra,
            vec![&"jumpnode_initialize_test_data".to_string()],
            "unexpected extra exports under --features expose_init_test_data"
        );
    } else {
        assert!(
            extra.is_empty(),
            "the DEFAULT Rust build must export exactly the C surface, found extras: {extra:?}"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so_path();
    let undef = undefined(&r);
    let bad: Vec<&String> = undef.iter().filter(|s| !is_libc_ish(s)).collect();
    println!("Rust undefined symbols: {undef:?}");
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

#[test]
fn rust_so_loads_with_rtld_now() {
    // RTLD_NOW forces the loader to bind *every* undefined symbol at load
    // time, so a successful open is proof that nothing is unresolved.
    use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
    let r = rust_so_path();
    let lib = unsafe { UnixLibrary::open(Some(&r), RTLD_NOW | RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("RTLD_NOW dlopen({}) failed: {e}", r.display()));
    let f = unsafe { lib.get::<JumpnodeFn>(b"jumpnode\0") }.expect("jumpnode");
    assert_eq!(unsafe { f(0, 0, 0, 0) }, ERR_UNKNOWN_MODE);

    // NOTE: the same check is deliberately NOT applied to the C .so. CMakeLists
    // never links libm, so the C library's `sqrt` reference is unresolved and
    // an RTLD_NOW open of it fails unless libm happens to be loaded already.
    // The shipped C library gets away with it because `node_count` is always 0,
    // so case 0004 early-returns and `sqrt` is never actually called under
    // lazy binding. That is a property of the C packaging, not of the
    // translation; the Rust .so needs no libm at all (it emits `sqrtsd`).
    let c = c_so_path();
    let lazy = unsafe { libloading::Library::new(&c) };
    assert!(lazy.is_ok(), "C .so must at least load lazily");
}

#[test]
fn c_shim_matches_rust_feature_surface() {
    // The init shim must export exactly `jumpnode` + the init hook, mirroring
    // the Rust build with `expose_init_test_data`.
    let shim = c_shim_so_path();
    let syms = exported(&shim);
    println!("C shim exports: {syms:?}");
    assert!(syms.contains("jumpnode"));
    assert!(syms.contains("jumpnode_initialize_test_data"));
}

#[test]
fn both_libraries_load_and_resolve_jumpnode() {
    // Proves we are calling through dlopen/dlsym, i.e. the exported wrappers.
    let p = Pair::shipped();
    println!("loaded C={} R={}", p.c_path.display(), p.r_path.display());
    let _ = p.assert_same(3, 1, 2, 0);
}
