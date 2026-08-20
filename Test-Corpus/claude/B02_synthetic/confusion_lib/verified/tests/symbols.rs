// Phase D — exported-symbol parity between the C .so and the Rust .so.
//
// Recomputes the `nm -D` diff of SYMBOLS.md at test time so the artifact can
// never silently go stale.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("running nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

/// Symbols the Rust cdylib exports purely because of the Rust runtime; they are
/// not part of the translated API surface and the C library has no counterpart.
fn is_rust_runtime_symbol(s: &str) -> bool {
    s.starts_with("_ZN")            // mangled Rust items
        || s.starts_with("__rust")
        || s.starts_with("rust_")
        || s == "_init"
        || s == "_fini"
        || s == "__bss_start"
        || s == "_edata"
        || s == "_end"
}

#[test]
fn symbols_c_exports_are_all_present_in_rust() {
    let c = defined_dynamic_symbols(&c_so_path());
    let rust = defined_dynamic_symbols(&rust_so_path());

    assert!(!c.is_empty(), "nm found no symbols in the C .so");

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   : {c:?}\n\
         Rust: {rust:?}"
    );

    // Documented surface (SYMBOLS.md).
    for expected in [
        "create_state",
        "destroy_state",
        "process_buffer",
        "update_flags",
        "confuse_types",
        "confusion",
    ] {
        assert!(c.contains(expected), "C .so is missing {expected}");
        assert!(rust.contains(expected), "Rust .so is missing {expected}");
    }
}

#[test]
fn symbols_rust_exports_no_unexpected_extras() {
    let c = defined_dynamic_symbols(&c_so_path());
    let rust = defined_dynamic_symbols(&rust_so_path());
    let extra: Vec<&String> = rust
        .difference(&c)
        .filter(|s| !is_rust_runtime_symbol(s))
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports non-runtime symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn symbols_rust_has_no_unresolvable_imports() {
    // If the Rust .so had an undefined non-libc symbol, dlopen would fail.
    // `impls()` dlopens both libraries and dlsyms every entry point.
    let (c, r) = impls();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");

    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(rust_so_path())
        .output()
        .expect("nm");
    let undefined: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    // Everything must come from libc / libgcc.
    for u in &undefined {
        let name = u.split('@').next().unwrap_or(u);
        assert!(
            !name.starts_with("_ZN") && !name.starts_with("confus"),
            "unresolved Rust-internal symbol in the cdylib: {u}"
        );
        assert!(
            !name.is_empty(),
            "unexpected empty symbol name in nm output"
        );
    }
}
