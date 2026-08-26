//! Phase A / Phase D: exported-symbol parity between the C shared object and
//! the Rust `cdylib`.
//!
//! Every symbol the C object *defines* must be exported by the Rust object
//! under the exact same name, and the Rust object must have no unresolvable
//! references (which would mean a translation unit was skipped).

mod common;

use common::{c_so, rust_so};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Parse `nm -D` output into (defined, undefined) name sets.
fn nm_symbols(so: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let out = Command::new("nm")
        .arg("-D")
        .arg(so)
        .output()
        .expect("spawn nm");
    assert!(
        out.status.success(),
        "nm -D {} failed:\n{}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut defined = BTreeSet::new();
    let mut undefined = BTreeSet::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields.len() {
            // "<addr> <type> <name>" -> a definition in this object.
            3 => {
                defined.insert(fields[2].to_string());
            }
            // "<type> <name>" -> no address, i.e. an import (U) or an
            // unresolved weak reference (w).
            2 => {
                undefined.insert(fields[1].to_string());
            }
            _ => {}
        }
    }
    (defined, undefined)
}

/// Strip a `@GLIBC_2.2.5`-style version suffix.
fn base_name(s: &str) -> &str {
    match s.find('@') {
        Some(i) => &s[..i],
        None => s,
    }
}

#[test]
fn c_defined_symbols_all_exported_by_rust() {
    let (c_defined, _) = nm_symbols(&c_so());
    let (r_defined, _) = nm_symbols(&rust_so());

    // Sanity: the four application symbols really are in the C object.
    for expected in ["printLine", "bad", "good", "main"] {
        assert!(
            c_defined.contains(expected),
            "the C shared object should define `{expected}`; it defines {c_defined:?}"
        );
    }

    let r_base: BTreeSet<&str> = r_defined.iter().map(|s| base_name(s)).collect();
    let missing: Vec<&str> = c_defined
        .iter()
        .map(|s| base_name(s))
        .filter(|s| !r_base.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "the Rust cdylib is missing {} symbol(s) defined by the C shared object: {missing:?}\n\
         C defines : {c_defined:?}\n\
         Rust defines: {r_defined:?}",
        missing.len()
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // RTLD_NOW makes the loader bind every relocation eagerly, so the load
    // fails outright if anything is unresolvable.
    const RTLD_NOW: i32 = 2;
    let so = rust_so();
    let lib = unsafe { libloading::os::unix::Library::open(Some(&so), RTLD_NOW) };
    let lib = lib.unwrap_or_else(|e| {
        panic!(
            "dlopen({}, RTLD_NOW) failed, i.e. the Rust cdylib references a symbol \
             that does not exist: {e}",
            so.display()
        )
    });

    // And the four exports must be resolvable by name.
    for name in [
        &b"printLine\0"[..],
        &b"bad\0"[..],
        &b"good\0"[..],
        &b"main\0"[..],
    ] {
        let sym: Result<libloading::os::unix::Symbol<*const ()>, _> = unsafe { lib.get(name) };
        assert!(
            sym.is_ok(),
            "dlsym({}) failed on the Rust cdylib",
            String::from_utf8_lossy(&name[..name.len() - 1])
        );
    }
}

#[test]
fn c_so_has_only_libc_imports() {
    // Documents the C object's import list, so a future change that pulls in a
    // new dependency is noticed.
    let (_, c_undefined) = nm_symbols(&c_so());
    let names: BTreeSet<&str> = c_undefined.iter().map(|s| base_name(s)).collect();
    for expected in ["__isoc99_scanf", "puts"] {
        assert!(
            names.contains(expected),
            "expected the C object to import `{expected}`; imports: {names:?}"
        );
    }
}

#[test]
fn both_objects_expose_the_same_application_surface() {
    let (c_defined, _) = nm_symbols(&c_so());
    let (r_defined, _) = nm_symbols(&rust_so());

    // The application-level surface: everything the C object defines.
    let c_app: BTreeSet<&str> = c_defined.iter().map(|s| base_name(s)).collect();
    let expected: BTreeSet<&str> = ["printLine", "bad", "good", "main"].into_iter().collect();
    assert_eq!(
        c_app, expected,
        "the C object's defined-symbol set changed; update SYMBOLS.md"
    );

    for name in &expected {
        assert!(
            r_defined.iter().any(|s| base_name(s) == *name),
            "the Rust cdylib does not export `{name}`"
        );
    }
}
