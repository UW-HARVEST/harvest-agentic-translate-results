//! Level 4: ABI surface — the Rust `.so` must export every dynamic symbol the
//! C `.so` exports, under the exact same name.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join("libdriver.so"));
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join("libdriver.so"));
            }
        }
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    candidates.push(target.join("debug/libdriver.so"));
    candidates.push(target.join("release/libdriver.so"));
    candidates
        .into_iter()
        .find(|p| p.exists())
        .expect("Rust cdylib not found")
}

/// Defined (exported) dynamic symbols, excluding the toolchain/libc boilerplate
/// that a Rust cdylib or a C shared object adds on its own.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // Either "<addr> <type> <name>" or "<type> <name>" (undefined addr).
            let (ty, name) = match it.next() {
                Some(name) => (b, name),
                None => (a, b),
            };
            // Keep only real code/data exports.
            if !matches!(ty, "T" | "t" | "D" | "B" | "R" | "W" | "i") {
                return None;
            }
            Some(name.to_string())
        })
        .filter(|n| !is_toolchain_symbol(n))
        .collect()
}

fn is_toolchain_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "rust_eh_personality",
        "__rust_alloc",
        "__rust_dealloc",
        "__rust_realloc",
        "__rust_alloc_zeroed",
        "__rust_alloc_error_handler",
        "__rust_alloc_error_handler_should_panic",
        "__rust_no_alloc_shim_is_unstable",
        "__rust_no_alloc_shim_is_unstable_v2",
        "__rdl_alloc",
        "__rdl_dealloc",
        "__rdl_realloc",
        "__rdl_alloc_zeroed",
        "__rg_oom",
    ];
    EXACT.contains(&name)
        || name.starts_with("_ZN")
        || name.starts_with("__rust_probestack")
        || name.starts_with("rust_metadata")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = exported_symbols(&root().join("c_src/build/libdriver.so"));
    let r = exported_symbols(&rust_so());

    assert!(
        c.contains("parse_number"),
        "sanity: C .so should export parse_number, got {c:?}"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C: {c:?}\n  Rust: {r:?}"
    );
}

#[test]
fn parse_number_is_dynamically_loadable_from_both() {
    // Redundant with the harness, but asserts the symbol resolves by exact name.
    unsafe {
        let c = libloading::Library::new(root().join("c_src/build/libdriver.so")).unwrap();
        let r = libloading::Library::new(rust_so()).unwrap();
        let _: libloading::Symbol<unsafe extern "C" fn(*mut u8, *mut u8) -> i32> =
            c.get(b"parse_number\0").unwrap();
        let _: libloading::Symbol<unsafe extern "C" fn(*mut u8, *mut u8) -> i32> =
            r.get(b"parse_number\0").unwrap();
    }
}
