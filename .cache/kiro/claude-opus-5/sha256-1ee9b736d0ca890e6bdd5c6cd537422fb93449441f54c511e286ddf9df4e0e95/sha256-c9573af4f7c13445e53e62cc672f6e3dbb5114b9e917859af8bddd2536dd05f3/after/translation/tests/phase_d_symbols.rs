//! Phase D — symbol parity gate, enforced as a test.
//!
//! Runs `nm -D` on both shared objects and fails if the C `.so` exports any
//! dynamic symbol the Rust `.so` does not. Also proves every C export is
//! actually resolvable through `dlopen`/`dlsym` on the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().expect("no .so in c_src/build")
}

fn rust_so() -> PathBuf {
    let mut probes = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile) = exe.parent().and_then(|d| d.parent()) {
            probes.push(profile.join("libcleanup_lib.so"));
        }
    }
    let target = workspace_root().join("translation").join("target");
    probes.push(target.join("release").join("libcleanup_lib.so"));
    probes.push(target.join("debug").join("libcleanup_lib.so"));
    probes
        .into_iter()
        .find(|p| p.exists())
        .expect("libcleanup_lib.so not built")
}

/// `nm -D` names, filtered by symbol-type predicate.
fn nm(so: &Path, extra: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .args(extra)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn every_c_export_is_exported_by_rust() {
    let c = nm(&c_so(), &["--defined-only"]);
    let r = nm(&rust_so(), &["--defined-only"]);

    assert!(
        c.contains("cleanup") && c.contains("print_result") && c.contains("cleanup_resources"),
        "sanity: the C .so should export the three known functions, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C export(s): {missing:?}\n\
         C exports:    {c:?}",
        missing.len()
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Every undefined symbol must be resolvable from the already-loaded
    // process image (libc / libgcc / loader). If any is not, `dlopen` of the
    // Rust .so would itself have failed, so loading it is the check.
    let _ = common::pair();

    // Additionally: no undefined symbol may be one of the library's own
    // exports (that would mean an export was declared but never defined).
    let undef = nm(&rust_so(), &["--undefined-only"]);
    for name in ["cleanup", "print_result", "cleanup_resources"] {
        assert!(
            !undef.contains(name),
            "`{name}` is undefined in the Rust .so — it is imported, not implemented"
        );
    }
}

#[test]
fn all_c_exports_resolve_via_dlsym_on_rust_so() {
    // `common::pair()` dlopens the Rust .so and dlsyms all three symbols; if
    // any were absent it panics with the symbol name.
    let p = common::pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
}
