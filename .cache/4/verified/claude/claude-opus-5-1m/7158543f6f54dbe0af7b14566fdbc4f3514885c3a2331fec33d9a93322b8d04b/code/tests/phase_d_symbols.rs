//! Phase D — symbol parity between the C and Rust shared objects.
//!
//! The gate is: **every symbol the C `.so` exports, the Rust `.so` must export
//! under the exact same name**, and the Rust `.so` must have no unresolved
//! non-libc symbols. The diff has to reach empty.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Runs `nm` with the given flags and returns the symbol names it printed.
fn nm(path: &Path, flags: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(flags)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm: {e}"));
    assert!(
        out.status.success(),
        "nm {flags:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn defined_dynamic(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
}

/// Symbols the toolchain emits into every object regardless of the source, and
/// which therefore carry no API meaning.
fn is_toolchain_boilerplate(s: &str) -> bool {
    matches!(
        s,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__gmon_start__"
            | "__cxa_finalize"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__odr_asan_gen___rust_no_alloc_shim_is_unstable"
    )
}

/// **The Phase D gate.** Every dynamic symbol defined by the C `.so` must also
/// be defined by the Rust `.so`, byte-for-byte the same name.
#[test]
fn c_defined_symbols_are_all_exported_by_rust() {
    let c = defined_dynamic(&c_so());
    let r = defined_dynamic(&rust_so());

    assert!(
        !c.is_empty(),
        "no symbols found in the C .so — the comparison would be vacuous"
    );

    let missing: Vec<&String> = c
        .iter()
        .filter(|s| !r.contains(*s) && !is_toolchain_boilerplate(s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}",
        missing.len()
    );
}

/// The C source's complete API is `driver` + `main`; assert both sides export
/// exactly that, so an accidental extra/renamed export is caught too.
#[test]
fn both_export_exactly_driver_and_main() {
    let want: BTreeSet<String> = ["driver", "main"].iter().map(|s| s.to_string()).collect();
    for (name, path) in [("C", c_so()), ("Rust", rust_so())] {
        let got: BTreeSet<String> = defined_dynamic(&path)
            .into_iter()
            .filter(|s| !is_toolchain_boilerplate(s))
            .collect();
        assert_eq!(
            got, want,
            "{name} .so exports an unexpected symbol set ({})",
            path.display()
        );
    }
}

/// Both exported symbols must actually be *callable* through `dlsym`, not just
/// present in the symbol table.
#[test]
fn both_exports_resolve_via_dlsym() {
    for lib in [c_lib(), rust_lib()] {
        assert!(
            lib.has_symbol(b"driver\0"),
            "{}: dlsym(driver) failed",
            lib.name
        );
        assert!(lib.has_symbol(b"main\0"), "{}: dlsym(main) failed", lib.name);
    }
}

/// The Rust `.so` must have **no unresolved non-libc symbols**: `ldd -r`
/// resolves every relocation and reports any that cannot be satisfied.
#[test]
fn rust_so_has_no_undefined_symbols() {
    for (name, path) in [("C", c_so()), ("Rust", rust_so())] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(&path)
            .output()
            .unwrap_or_else(|e| panic!("failed to run ldd: {e}"));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.to_lowercase().contains("undefined symbol"))
            .collect();
        assert!(
            bad.is_empty(),
            "{name} .so ({}) has undefined symbols:\n{}",
            path.display(),
            bad.join("\n")
        );
    }
}

/// Both *executables* must provide the entry symbol a loader looks for.
#[test]
fn both_executables_define_main() {
    for (name, path) in [("C", c_exe()), ("Rust", rust_exe_release())] {
        let syms = nm(&path, &["--defined-only"]);
        assert!(
            syms.contains("main"),
            "{name} executable ({}) does not define `main`",
            path.display()
        );
    }
}

/// `SYMBOLS.md` must stay in sync with reality: every symbol `nm` reports for
/// the C `.so` has to be documented there.
#[test]
fn symbols_md_documents_every_symbol() {
    let doc = std::fs::read_to_string(crate_root().join("SYMBOLS.md")).expect("read SYMBOLS.md");
    for s in defined_dynamic(&c_so()) {
        if is_toolchain_boilerplate(&s) {
            continue;
        }
        assert!(
            doc.contains(&format!("`{s}`")),
            "SYMBOLS.md does not document the exported symbol `{s}`"
        );
    }
}

/// Guard against a vacuous suite: the C `.so` we compare against must really be
/// built from `c_src/src/main.c`, and it must be a *different* file from the
/// Rust one.
#[test]
fn artifacts_are_distinct_and_present() {
    let c = c_so();
    let r = rust_so();
    assert!(Path::new(&c).is_file(), "C .so missing: {c:?}");
    assert!(Path::new(&r).is_file(), "Rust .so missing: {r:?}");
    assert_ne!(c, r);
    assert!(
        c_source().is_file(),
        "the C source vanished: {:?}",
        c_source()
    );
}
