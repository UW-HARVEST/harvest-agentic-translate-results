// Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Only global text/data definitions; skip weak/local artefacts.
            if matches!(kind, "T" | "D" | "B" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            if kind == "U" {
                Some(name.split('@').next().unwrap().to_string())
            } else {
                None
            }
        })
        .collect()
}

/// The six symbols the C source defines, spelled out literally so the test
/// fails if the C `.so` ever stops exporting one of them too.
const EXPECTED: [&str; 6] = [
    "confuse_types",
    "confusion",
    "create_state",
    "destroy_state",
    "process_buffer",
    "update_flags",
];

#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();
    let c = exported(&c_path);
    let r = exported(&r_path);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C  = {}\nRust = {}",
        c_path.display(),
        r_path.display()
    );

    for name in EXPECTED {
        assert!(c.contains(name), "C .so does not export {name}");
        assert!(r.contains(name), "Rust .so does not export {name}");
    }
    assert_eq!(
        c.len(),
        EXPECTED.len(),
        "C .so exports an unexpected symbol set: {c:?}"
    );
}

#[test]
fn every_symbol_is_dlsym_resolvable_in_both() {
    // Opening both libraries resolves all six symbols through dlsym or panics.
    let (c, r) = common::both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
    assert_ne!(c.path, r.path);
    assert_ne!(c.confusion as usize, r.confusion as usize);
}

#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let r_path = common::rust_so_path();
    let u = undefined(&r_path);

    // Everything the Rust object imports must be resolvable from the C runtime
    // (glibc) or libgcc; there must be no dangling Rust-side references.
    let out = Command::new("ldd").arg("-r").arg(&r_path).output().expect("ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "ldd -r reports unresolved symbols for {}:\n{text}",
        r_path.display()
    );

    // Sanity: the translation really does route I/O and allocation through libc
    // (that is what keeps printf formatting byte-identical).
    for required in ["printf", "snprintf", "malloc", "free", "strlen", "memchr"] {
        assert!(
            u.contains(required),
            "Rust .so does not import libc `{required}` (imports: {u:?})"
        );
    }
}
