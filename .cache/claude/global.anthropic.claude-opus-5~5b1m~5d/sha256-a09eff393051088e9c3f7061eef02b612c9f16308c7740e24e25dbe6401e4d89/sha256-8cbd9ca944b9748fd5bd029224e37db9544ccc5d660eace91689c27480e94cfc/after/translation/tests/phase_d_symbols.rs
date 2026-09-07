// Phase D — symbol parity between the C and Rust shared objects.
// Enforces the SYMBOLS.md conclusion as an executable check.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn dynamic_defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        // ignore the linker/runtime scaffolding that is not part of the API
        .filter(|s| {
            !matches!(
                s.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "_IO_stdin_used"
            ) && !s.starts_with("rust_")
                && !s.starts_with("__rust")
        })
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = dynamic_defined_symbols(&c_so_path());
    let r = dynamic_defined_symbols(rust_so_path());

    assert!(c.contains("driver"), "C .so must export `driver`, got {c:?}");

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn no_undefined_non_libc_symbols_in_rust() {
    // Everything the Rust .so imports must be resolvable from the C runtime it
    // is linked against (libc / libgcc / ld.so), i.e. nothing exotic.
    let allowed_libs = [
        "libc.so", "libm.so", "libgcc", "libdl", "libpthread", "ld-linux",
    ];
    let ldd = Command::new("ldd").arg(rust_so_path()).output().expect("ldd");
    let text = String::from_utf8_lossy(&ldd.stdout);
    assert!(
        !text.contains("not found"),
        "Rust .so has unresolved shared-library deps:\n{text}"
    );
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("linux-vdso") || line.starts_with("statically") {
            continue;
        }
        assert!(
            allowed_libs.iter().any(|l| line.contains(l)),
            "Rust .so pulls in an unexpected library: {line}"
        );
    }

    // And the undefined list must be a subset of what the C .so needs plus the
    // Rust std runtime's libc/pthread usage — in particular no leftover Rust
    // symbol that only exists in some other crate.
    let undef = undefined_symbols(rust_so_path());
    let suspicious: Vec<_> = undef
        .iter()
        .filter(|s| s.starts_with("_ZN") || s.contains("17h"))
        .cloned()
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has undefined mangled Rust symbols: {suspicious:?}"
    );
}

#[test]
fn c_source_tree_is_fully_translated() {
    // Guards against a whole C module having been skipped: assert the set of C
    // translation units still matches what SYMBOLS.md was derived from.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src");
    let mut cs: Vec<String> = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("read_dir") {
            let p = e.expect("dir entry").path();
            if p.is_dir() {
                if p.file_name().map(|n| n != "build").unwrap_or(true) {
                    stack.push(p);
                }
            } else if p.extension().map(|x| x == "c").unwrap_or(false) {
                cs.push(
                    p.strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    cs.sort();
    assert_eq!(
        cs,
        vec!["src/driver.c".to_string()],
        "the set of C translation units changed; SYMBOLS.md and the translation \
         must be revisited (a whole module may be untranslated)"
    );
}
