//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D --defined-only` on both libraries and asserts that every symbol
//! the C library exports is also exported by the Rust library under the exact
//! same name. Also proves the symbol is genuinely resolvable via `dlsym` (not
//! merely present in the table).

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    std::env::var("C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("c_src/build/libdriver.so"))
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let cand = exe
        .parent()
        .and_then(|d| d.parent())
        .map(|p| p.join("libdriver.so"));
    match cand {
        Some(p) if p.exists() => p,
        _ => manifest_dir().join("target/debug/libdriver.so"),
    }
}

/// Dynamic symbols DEFINED by a shared object, excluding Rust-mangled ones
/// (`_ZN…`, `_R…`) which are implementation detail, not part of the C ABI.
fn defined_dynamic_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` — is binutils installed?");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .filter(|s| !s.starts_with("_ZN") && !s.starts_with("_R"))
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn c_exported_symbols_are_all_exported_by_rust() {
    let c = c_so();
    let r = rust_so();
    assert!(c.exists(), "C .so missing at {c:?}");
    assert!(r.exists(), "Rust .so missing at {r:?}");

    let c_syms = defined_dynamic_symbols(&c);
    let r_syms = defined_dynamic_symbols(&r);

    assert!(
        !c_syms.is_empty(),
        "nm reported no defined dynamic symbols in the C .so — parsing bug"
    );
    assert!(
        c_syms.contains("UTIL_createLinePointers"),
        "C .so does not export UTIL_createLinePointers: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C-exported symbol(s): {missing:?}",
        missing.len()
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so();
    let out = Command::new("ldd")
        .args(["-r", r.to_str().unwrap()])
        .output()
        .expect("failed to run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved symbols:\n{}",
        bad.join("\n")
    );
}

#[test]
fn both_symbols_resolve_through_dlsym() {
    // Touching the harness forces a real dlopen + dlsym of BOTH libraries.
    let cf = common::c_create();
    let rf = common::rust_create();
    assert!(!(cf as usize == 0), "C symbol resolved to NULL");
    assert!(!(rf as usize == 0), "Rust symbol resolved to NULL");
    assert_ne!(
        cf as usize, rf as usize,
        "both dlsym lookups returned the same address — the two libraries were \
         not loaded independently, so nothing is actually being compared"
    );
}
