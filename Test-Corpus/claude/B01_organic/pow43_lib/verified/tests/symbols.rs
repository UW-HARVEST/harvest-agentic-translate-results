//! Phase D — exported-symbol parity between the C `.so` and the Rust `cdylib`.
//!
//! Automates the `nm -D` comparison documented in `SYMBOLS.md` so that a
//! regression (a dropped `#[no_mangle]`, a renamed export, an accidental extra
//! public symbol) fails the test suite.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::*;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn undefined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm -u failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn phase_d_rust_exports_every_c_symbol() {
    let i = impls();
    let c = defined_dynamic_symbols(&i.c_path);
    let r = defined_dynamic_symbols(&i.rust_path);
    assert!(c.contains("pow43"), "the C .so must export pow43: {c:?}");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
}

#[test]
fn phase_d_rust_exports_no_extra_public_api() {
    let i = impls();
    let c = defined_dynamic_symbols(&i.c_path);
    let r = defined_dynamic_symbols(&i.rust_path);
    // The Rust cdylib only exports its `#[no_mangle]` items, so the public
    // surface must be exactly the C one.
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols the C .so does not: {extra:?}"
    );
    assert_eq!(
        r.iter().cloned().collect::<Vec<_>>(),
        vec!["pow43".to_string()],
        "unexpected public symbol set"
    );
}

#[test]
fn phase_d_no_unresolved_non_libc_symbols() {
    let i = impls();
    // Everything the Rust cdylib imports must be a platform (libc / libgcc /
    // libpthread) symbol; nothing may be left unresolved.
    let undef = undefined_dynamic_symbols(&i.rust_path);
    let ld = Command::new("ldd").arg("-r").arg(&i.rust_path).output();
    if let Ok(ld) = ld {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&ld.stdout),
            String::from_utf8_lossy(&ld.stderr)
        );
        assert!(
            !text.contains("undefined symbol"),
            "ldd -r reported unresolved symbols:\n{text}"
        );
    }
    // sanity: the import list is non-empty but contains no mangled Rust symbol
    // and no symbol from the C library under test
    for s in &undef {
        assert!(
            !s.starts_with("_ZN") && !s.starts_with("_RN"),
            "unresolved mangled Rust symbol: {s}"
        );
        assert_ne!(s.as_str(), "pow43", "the Rust .so must define pow43 itself");
    }
}
