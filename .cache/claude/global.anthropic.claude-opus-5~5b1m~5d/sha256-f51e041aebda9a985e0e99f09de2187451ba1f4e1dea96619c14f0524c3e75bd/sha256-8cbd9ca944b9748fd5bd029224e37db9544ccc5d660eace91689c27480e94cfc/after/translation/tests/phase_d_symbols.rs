//! Phase D — symbol parity between the C `.so` and the Rust `.so`, enforced as
//! a test so it cannot silently regress.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    let build = root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    v.sort();
    v.into_iter().next().expect("C .so not built")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    for p in ["release", "debug"] {
        let c = root()
            .join("translation")
            .join("target")
            .join(p)
            .join("libgaussian_kernel_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("Rust cdylib not built")
}

/// Symbols that are produced by the linker / language runtime rather than by the
/// library's own source, and therefore excluded from the parity comparison.
fn is_runtime_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "_edata"
            | "_end"
            | "__bss_start"
            | "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__gmon_start__"
            | "__cxa_finalize"
            | "rust_eh_personality"
    ) || name.starts_with("__rust")
        || name.starts_with("_R")
        || name.starts_with("_ZN")
}

fn exported(path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            let (kind, name) = match (a, b, c) {
                (Some(_addr), Some(k), Some(n)) => (k, n),
                (Some(k), Some(n), None) => (k, n),
                _ => return None,
            };
            // Only global/weak code & data symbols.
            if !matches!(kind, "T" | "W" | "D" | "B" | "R" | "i" | "V") {
                return None;
            }
            if is_runtime_symbol(name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_so());
    let r = exported(&rust_so());
    assert!(
        c.contains(&"gaussian_kernel".to_string()),
        "sanity: C .so must export gaussian_kernel, got {c:?}"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}\n  C:    {c:?}\n  Rust: {r:?}"
    );
}

#[test]
fn rust_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so().to_str().unwrap()])
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let allowed_prefixes = ["__", "_ITM_", "_Unwind", "pthread_"];
    let mut bad = Vec::new();
    for line in text.lines() {
        let raw = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        // Strip the ELF symbol-version suffix, e.g. `expf@GLIBC_2.27` -> `expf`.
        let (name, version) = match raw.split_once('@') {
            Some((n, v)) => (n, Some(v)),
            None => (raw, None),
        };
        // Anything bound to a glibc/libgcc version node comes from libc/libm.
        if version
            .map(|v| v.starts_with("GLIBC") || v.starts_with("GCC") || v.starts_with("GLIBCXX"))
            .unwrap_or(false)
        {
            continue;
        }
        if allowed_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        bad.push(name.to_string());
    }
    assert!(bad.is_empty(), "unexpected undefined symbols in Rust .so: {bad:?}");
}

#[test]
fn rust_so_exports_expf_dependency_on_platform_libm() {
    // The Rust library must *import* expf (i.e. use the same libm the C uses),
    // not inline its own exp implementation, or bit-exactness is accidental.
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so().to_str().unwrap()])
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.lines().any(|l| {
            l.split_whitespace()
                .last()
                .map(|s| s.split('@').next() == Some("expf"))
                .unwrap_or(false)
        }),
        "Rust .so must import expf from libm; undefined symbols were:\n{text}"
    );
}
