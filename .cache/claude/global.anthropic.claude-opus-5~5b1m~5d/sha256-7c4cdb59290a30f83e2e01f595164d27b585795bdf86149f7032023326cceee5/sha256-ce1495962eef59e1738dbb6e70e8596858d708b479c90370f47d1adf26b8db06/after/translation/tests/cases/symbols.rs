// Phase D -- symbol parity, enforced as a test so it cannot silently regress.

use crate::common::*;

use std::collections::BTreeSet;
use std::process::Command;

/// Defined dynamic symbol names from `nm -D --defined-only`, ignoring the
/// non-public data/read-only entries a Rust cdylib may emit.
pub fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Public code/data exports only.
            if matches!(kind, "T" | "t" | "D" | "B" | "W" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

pub fn every_c_symbol_is_exported_by_rust() {
    let i = impls();
    let c = defined_symbols(&i.c_path);
    let r = defined_symbols(&i.rust_path);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C .so:    {} -> {c:?}\n Rust .so: {} -> {r:?}",
        i.c_path.display(),
        i.rust_path.display()
    );

    // The C library's entire public surface is the single `driver` symbol.
    assert!(c.contains("driver"), "C .so must export `driver`, got {c:?}");
    assert_eq!(
        c.len(),
        1,
        "expected the C .so to export exactly one symbol; if this changed, a new \
         C source file appeared and must be translated: {c:?}"
    );
    assert!(r.contains("driver"), "Rust .so must export `driver`, got {r:?}");
}

/// The Rust `.so` must have no unresolved symbols -- i.e. nothing that failed to
/// be emitted on the Rust side. Everything undefined must come from libc/libgcc.
pub fn rust_so_has_no_unresolved_symbols() {
    let i = impls();
    let out = Command::new("ldd")
        .arg("-r")
        .arg(&i.rust_path)
        .output()
        .expect("run ldd -r");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol"))
        .collect();
    assert!(bad.is_empty(), "Rust .so has unresolved symbols: {bad:?}");
}

/// `dlsym` must find `driver` in both libraries with the exact same, unmangled
/// name -- which the harness already relies on, asserted explicitly here.
pub fn driver_symbol_is_callable_from_both_libraries() {
    let i = impls();
    let c = capture_stdout(|| unsafe { (i.c)(1.0) });
    let r = capture_stdout(|| unsafe { (i.rust)(1.0) });
    assert_eq!(c, r);
    assert_eq!(String::from_utf8_lossy(&c), "3ff0000000000000 0x1p+0 1.0000\n");
}
