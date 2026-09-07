//! Phase D: mechanical `nm -D` symbol-parity check between the C `.so` and the
//! Rust `.so`. The diff must be empty.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    std::env::var("DRIVER_C_SO").map(PathBuf::from).unwrap_or_else(|_| {
        manifest_dir()
            .parent()
            .unwrap()
            .join("c_src/build/libdriver.so")
    })
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let r = manifest_dir().join("target/release/libdriver.so");
    if r.exists() {
        r
    } else {
        manifest_dir().join("target/debug/libdriver.so")
    }
}

/// Defined, exported ("T"/"D"/"B"/"R") dynamic symbols, excluding the libc /
/// compiler-runtime plumbing that a Rust cdylib unavoidably re-exports.
fn exported(path: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = match (it.next(), it.next(), it.next()) {
                (Some(a), Some(k), Some(n)) => (a, k, n),
                _ => return None,
            };
            if !matches!(kind, "T" | "D" | "B" | "R" | "t") {
                return None;
            }
            Some(name.to_string())
        })
        .filter(|n| !is_runtime_symbol(n))
        .collect()
}

/// Symbols that belong to the language runtime / linker, not to the library's
/// own API surface.
fn is_runtime_symbol(n: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "rust_eh_personality",
        "_Unwind_Resume",
    ];
    EXACT.contains(&n)
        || n.starts_with("_ZN")            // Rust/C++ mangled internals
        || n.starts_with("_R")             // Rust v0 mangling
        || n.starts_with("__rust")
        || n.starts_with("rust_")
        || n.starts_with("__rdl_")
        || n.starts_with("_Unwind")
        || n.starts_with("__gxx")
        || n.starts_with("_GLOBAL__")
}

#[test]
fn nm_symbol_diff_is_empty() {
    let c = exported(&c_so());
    let r = exported(&rust_so());

    assert!(!c.is_empty(), "nm found no exported symbols in the C .so");

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );

    // The library's five public functions, spelled out, so a regression in the
    // filtering logic above cannot silently make this test vacuous.
    for want in ["driver", "bad", "good", "printIntLine", "printLine"] {
        assert!(c.contains(want), "C .so must export {want}");
        assert!(r.contains(want), "Rust .so must export {want}");
    }

    // Report (but do not fail on) any extra API-looking symbols in Rust.
    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports extra non-runtime symbols the C .so does not: {extra:?}"
    );
}
