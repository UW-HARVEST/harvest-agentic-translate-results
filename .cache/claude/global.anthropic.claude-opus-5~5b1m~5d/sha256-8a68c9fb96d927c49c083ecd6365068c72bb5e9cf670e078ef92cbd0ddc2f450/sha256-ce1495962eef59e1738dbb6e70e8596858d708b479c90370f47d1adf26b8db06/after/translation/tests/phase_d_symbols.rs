//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::impls;
use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Guards against the whole suite silently degrading to "C vs nothing" or to a
/// single stale profile: if both Rust profiles are on disk, both must be loaded.
#[test]
fn harness_covers_every_rust_profile_present() {
    let impls = impls();
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(!impls.rust.is_empty(), "no Rust .so under test");

    if std::env::var_os("RUST_DRIVER_SO").is_none() {
        for rel in ["target/release/libdriver.so", "target/debug/libdriver.so"] {
            let p = manifest.join(rel);
            if p.exists() {
                assert!(
                    impls.rust.iter().any(|r| r.path == p),
                    "{rel} exists on disk but the harness did not load it"
                );
            }
        }
    }
    eprintln!(
        "harness is comparing C against {} Rust .so: {:?}",
        impls.rust.len(),
        impls.rust.iter().map(|r| r.name).collect::<Vec<_>>()
    );
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let impls = impls();
    let c_syms = defined_dynamic_symbols(&impls.c.path);
    assert!(
        c_syms.contains(&"driver".to_string()),
        "sanity: C .so should export `driver`, got {c_syms:?}"
    );

    for r in &impls.rust {
        let rust_syms = defined_dynamic_symbols(&r.path);
        let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "{} is missing symbols exported by the C .so: {missing:?}",
            r.name
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Every undefined symbol must be satisfiable by the platform: glibc or
    // libgcc's unwinder. Anything else would mean a dangling reference.
    for r in &impls().rust {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", r.path.to_str().unwrap()])
            .output()
            .expect("failed to run `nm`");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        let suspicious: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().last())
            .filter(|s| {
                // glibc-versioned imports, the ITM/gmon weak markers and the
                // unwinder are all provided by the platform.
                !s.contains("@GLIBC")
                    && !s.contains("@GCC")
                    && !s.starts_with("_ITM_")
                    && !s.starts_with("__gmon_start__")
                    && !s.starts_with("_Unwind_")
            })
            .collect();
        assert!(
            suspicious.is_empty(),
            "{} has unresolved non-libc symbols: {suspicious:?}",
            r.name
        );
    }
}

#[test]
fn driver_symbol_is_loadable_from_both() {
    // Redundant with the harness's load-time probe, but makes the parity claim
    // explicit: the symbol resolves and is callable through dlsym in both.
    let impls = impls();
    let _ = impls.c.driver();
    for r in &impls.rust {
        let _ = r.driver();
    }
}

#[test]
fn rust_so_does_not_export_extra_public_api() {
    // Not a hard requirement (the Rust std runtime may add its own symbols),
    // but any *additional* symbol that looks like a public C API would signal an
    // accidental extra export. Report it for visibility.
    let impls = impls();
    let c_syms = defined_dynamic_symbols(&impls.c.path);
    for r in &impls.rust {
        let extra: Vec<String> = defined_dynamic_symbols(&r.path)
            .into_iter()
            .filter(|s| !c_syms.contains(s) && !s.starts_with('_'))
            .collect();
        assert!(
            extra.is_empty(),
            "{} exports unmangled symbols the C .so does not: {extra:?}",
            r.name
        );
    }
}
