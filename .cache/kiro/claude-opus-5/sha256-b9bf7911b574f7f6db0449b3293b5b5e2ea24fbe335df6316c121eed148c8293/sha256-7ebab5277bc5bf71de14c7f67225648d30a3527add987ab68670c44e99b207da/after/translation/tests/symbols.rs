//! Every dynamic symbol the C `libdriver.so` exports must also be exported by
//! the Rust `libdriver.so`, under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{FnDriver, FnPrintLine, FnVoid, c_so_path, impls, rust_so_path, sym};

/// Names `nm -D` reports as *defined* (uppercase type letter) for `path`.
/// Weak/undefined entries are skipped: they are toolchain runtime hooks, not
/// part of the library's API surface.
fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .expect("`nm` must be available to compare exported symbols");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8_lossy(&out.stdout);
    let mut names = BTreeSet::new();
    for line in text.lines() {
        // Format: "<addr> <type> <name>" or "<spaces> <type> <name>".
        let mut fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 2 {
            continue;
        }
        let name = fields.pop().unwrap().to_string();
        let kind = fields.pop().unwrap();
        let defined = kind.len() == 1
            && kind
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase() && c != 'U');
        if defined {
            names.insert(name);
        }
    }
    names
}

/// Strips glibc/compiler-runtime bookkeeping that is not part of driver.c.
fn is_runtime_symbol(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("_Z")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_R")
        || matches!(
            name,
            "_init"
                | "_fini"
                | "_edata"
                | "_end"
                | "__bss_start"
                | "__gmon_start__"
                | "__gnu_lto_slim"
        )
}

#[test]
fn c_exports_the_expected_api() {
    // Guards against silently comparing against an empty/stale C library.
    let c_syms = defined_dynamic_symbols(&c_so_path());
    for expected in ["printLine", "bad", "good", "driver"] {
        assert!(
            c_syms.contains(expected),
            "C library does not export `{expected}`; found: {c_syms:?}"
        );
    }
}

#[test]
fn rust_so_exports_every_c_so_symbol() {
    let c_path = c_so_path();
    let rust_path = rust_so_path();

    let c_syms = defined_dynamic_symbols(&c_path);
    let rust_syms = defined_dynamic_symbols(&rust_path);

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|n| !is_runtime_symbol(n))
        .filter(|n| !rust_syms.contains(*n))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust library is missing exports present in the C library: {missing:?}\n\
         C   ({}) : {:?}\n\
         Rust({}) : {:?}",
        c_path.display(),
        c_syms,
        rust_path.display(),
        rust_syms,
    );
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_the_rust_so() {
    // nm parity is necessary but not sufficient — confirm dlsym actually
    // resolves each name against the Rust object.
    let libs = impls();
    let _: libloading::Symbol<FnPrintLine> = sym(libs.rust, "printLine");
    let _: libloading::Symbol<FnVoid> = sym(libs.rust, "bad");
    let _: libloading::Symbol<FnVoid> = sym(libs.rust, "good");
    let _: libloading::Symbol<FnDriver> = sym(libs.rust, "driver");

    // And the same names must resolve in the C object, so the tests above are
    // really comparing like with like.
    let _: libloading::Symbol<FnPrintLine> = sym(libs.c, "printLine");
    let _: libloading::Symbol<FnVoid> = sym(libs.c, "bad");
    let _: libloading::Symbol<FnVoid> = sym(libs.c, "good");
    let _: libloading::Symbol<FnDriver> = sym(libs.c, "driver");
}

#[test]
fn rust_so_does_not_export_internal_helpers() {
    // `helperBad` / `helperGood1` are `static` in C, so they must not appear in
    // either library's dynamic symbol table.
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let rust_syms = defined_dynamic_symbols(&rust_so_path());
    for hidden in ["helperBad", "helperGood1"] {
        assert!(!c_syms.contains(hidden), "C unexpectedly exports {hidden}");
        assert!(
            !rust_syms.contains(hidden),
            "Rust unexpectedly exports {hidden}"
        );
    }
}
