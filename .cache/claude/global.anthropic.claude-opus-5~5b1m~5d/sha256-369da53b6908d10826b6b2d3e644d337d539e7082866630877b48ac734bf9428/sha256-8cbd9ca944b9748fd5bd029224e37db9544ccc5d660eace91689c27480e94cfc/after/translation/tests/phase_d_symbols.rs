//! Phase D — symbol parity gate, enforced as a test.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_so(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    if !names.is_empty() {
        for n in names {
            let p = dir.join(n);
            if p.exists() {
                return Some(p);
            }
        }
        return None;
    }
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    v.sort();
    v.into_iter().next()
}

/// Linker/loader-generated names that are not part of any C API surface.
const IGNORED: &[&str] = &["_init", "_fini", "__bss_start", "_edata", "_end"];

fn defined_globals(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only strong global code/data symbols.
            if !matches!(kind, "T" | "D" | "B" | "R") {
                return None;
            }
            let name = name.split('@').next().unwrap_or(name);
            if IGNORED.contains(&name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c_so = find_so(&root().join("c_src/build"), &[]).expect("C .so — build c_src first");

    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let rust_so = find_so(&profile_dir, &["libbitwriter_add_lib.so", "libtranslation.so"])
        .or_else(|| find_so(&root().join("translation/target/release"), &["libbitwriter_add_lib.so"]))
        .expect("Rust cdylib — run cargo build first");

    let c_syms = defined_globals(&c_so);
    let rust_syms = defined_globals(&rust_so);

    assert!(!c_syms.is_empty(), "nm found no C symbols — bad parse");
    assert!(
        c_syms.contains("bitwriter_add"),
        "C .so is missing bitwriter_add: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {:?}\n  C   = {:?}\n  Rust= {:?}",
        missing.len(),
        missing,
        c_syms,
        rust_syms
    );
}

/// No undefined non-libc symbols in the Rust `.so` (nothing left dangling by a
/// partially translated module).
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let rust_so = find_so(&profile_dir, &["libbitwriter_add_lib.so", "libtranslation.so"])
        .or_else(|| find_so(&root().join("translation/target/release"), &["libbitwriter_add_lib.so"]))
        .expect("Rust cdylib");

    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace().peekable();
            // `nm -D --undefined-only` prints "<blank addr> U name" or "w name".
            let kind = it.next()?;
            let name = it.next()?;
            Some((kind.to_string(), name.to_string()))
        })
        .filter(|(kind, name)| {
            // Weak undefined symbols are optional by definition (the loader
            // leaves them null) — never a translation gap.
            if kind == "w" {
                return false;
            }
            // Anything carrying a version tag (@GLIBC_x / @GCC_x) is satisfied
            // by libc / libgcc, i.e. a legitimate system import.
            if name.contains("@GLIBC") || name.contains("@GCC") || name.contains("@CXXABI") {
                return false;
            }
            let bare = name.split('@').next().unwrap_or(name);
            // Compiler-runtime / unwinder / TLS internals.
            !(bare.starts_with("__")
                || bare.starts_with("_ITM_")
                || bare.starts_with("_Unwind")
                || bare.starts_with("pthread_")
                || bare.starts_with("rust_eh_personality"))
        })
        .map(|(k, n)| format!("{k} {n}"))
        .collect();
    assert!(bad.is_empty(), "unresolved non-libc symbols in Rust .so: {bad:?}");
}

/// Sanity: the symbol loaded from each `.so` is actually callable and the two
/// agree on a trivial input (guards against a mis-resolved dlsym).
#[test]
fn both_symbols_callable_via_dlsym() {
    check_call(Bitwriter::zeroed(), 8, 0xAB, "phase-d-smoke");
}
