//! Phase D — symbol parity, enforced as a test so it cannot drift.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and every symbol the Rust `.so` imports must
//! resolve at load time.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest().parent().unwrap().join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    common::rust_so_path()
}

/// Dynamic symbols of a given nm type-class filter.
fn nm(so: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next(), it.next());
            match (a, b) {
                // "<addr> <type> <name>"
                (Some(_addr), Some(ty)) if ty.len() == 1 => {
                    it.next().map(|n| n.split('@').next().unwrap().to_string())
                }
                // "<type> <name>" (undefined / weak, no address)
                (Some(ty), Some(name)) if ty.len() == 1 => {
                    Some(name.split('@').next().unwrap().to_string())
                }
                _ => None,
            }
        })
        .collect()
}

#[test]
fn d1_rust_exports_every_c_symbol() {
    let c = nm(&c_so(), "--defined-only");
    let r = nm(&rust_so(), "--defined-only");
    assert!(!c.is_empty(), "C .so exported nothing — bad build?");
    assert!(c.contains(&"driver".to_string()), "C .so must export `driver`");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols: {missing:?}\n  C   = {c:?}\n  Rust = {r:?}"
    );
}

#[test]
fn d2_rust_imports_all_resolve() {
    // `ldd` reports "not found" for anything unresolvable at load time.
    let out = Command::new("ldd").arg(rust_so()).output().expect("run ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "unresolved shared-object dependency:\n{text}"
    );

    // No undefined symbol may be a project symbol: they must all be libc /
    // libgcc / weak loader stubs.
    let undef = nm(&rust_so(), "--undefined-only");
    let suspicious: Vec<&String> = undef
        .iter()
        .filter(|s| s.contains("driver") || s.starts_with("_ZN"))
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc symbols: {suspicious:?}"
    );
}

#[test]
fn d3_driver_symbol_is_callable_from_both() {
    // Sanity: the exported symbol resolves via dlsym in both libraries and both
    // produce the same bytes (full coverage lives in phases B and C).
    for x in [0, 1, -1, i32::MIN, i32::MAX] {
        common::assert_same(x, "D3");
    }
}
