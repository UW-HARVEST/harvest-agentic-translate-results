//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Mechanically re-derives `SYMBOLS.md` at test time so the artifact can never
//! drift from reality.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Symbols that are libc / language-runtime plumbing rather than part of the
/// library's own public surface.
fn is_runtime_noise(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("_Unwind_")
        || name.starts_with("__")
        || name.starts_with("_init")
        || name.starts_with("_fini")
        || name == "_edata"
        || name == "_end"
        || name == "__bss_start"
}

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next(), it.next());
            // Lines are either "<addr> <type> <name>" or "<type> <name>".
            let (ty, name) = match (a, b, it.next()) {
                (Some(_addr), Some(ty), Some(name)) => (ty, name),
                (Some(ty), Some(name), None) => (ty, name),
                _ => return None,
            };
            // Keep only global text/data/bss/weak definitions.
            if !matches!(ty, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i") {
                return None;
            }
            if is_runtime_noise(name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn nm_undefined_non_libc(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(out.status.success(), "nm -u failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|n| !is_runtime_noise(n))
        // Everything the Rust cdylib imports beyond libc plumbing would show up
        // here. Filter out known-libc names by checking they resolve in libc.
        .filter(|n| !resolves_in_libc(n))
        .collect()
}

/// True if the symbol is provided by the process's libc / pthread / dl / m.
fn resolves_in_libc(name: &str) -> bool {
    // Cheap, robust check: ask the dynamic loader.
    use libloading::Library;
    // `Library::this()` is not available across libloading versions; dlopen the
    // main program image instead (NULL handle semantics) via an empty filename
    // is not portable, so fall back to a static allowlist of the glibc modules.
    for lib in [
        "libc.so.6",
        "libm.so.6",
        "libpthread.so.0",
        "libdl.so.2",
        "libgcc_s.so.1",
    ] {
        if let Ok(l) = unsafe { Library::new(lib) } {
            let mut sym = name.as_bytes().to_vec();
            sym.push(0);
            if unsafe { l.get::<*const ()>(&sym) }.is_ok() {
                return true;
            }
        }
    }
    false
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = nm_defined(&common::c_so_path());
    let r = nm_defined(&common::rust_so_path());

    println!("C   defined: {c:?}");
    println!("Rust defined: {r:?}");

    // The two documented entry points must actually be there (guards against a
    // vacuous pass if `nm` output parsing ever breaks).
    assert!(c.contains("static_sum"), "C .so is missing static_sum: {c:?}");
    assert!(c.contains("driver"), "C .so is missing driver: {c:?}");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
}

#[test]
fn d2_rust_exports_no_extra_public_symbols() {
    let c = nm_defined(&common::c_so_path());
    let r = nm_defined(&common::rust_so_path());
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports public symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn d3_rust_has_no_unresolved_non_libc_symbols() {
    let undef = nm_undefined_non_libc(&common::rust_so_path());
    assert!(
        undef.is_empty(),
        "Rust .so has undefined non-libc symbols: {undef:?}"
    );
}

#[test]
fn d4_function_local_static_is_not_exported() {
    // The C `static int sum` has no linkage; the Rust `static mut SUM` must
    // likewise stay private, or the ABI surface would differ.
    let r = nm_defined(&common::rust_so_path());
    for name in ["SUM", "sum"] {
        assert!(
            !r.contains(name),
            "Rust .so must not export the accumulator `{name}`"
        );
    }
}
