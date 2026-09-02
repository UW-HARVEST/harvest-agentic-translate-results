// Phase D — symbol parity between the two shared objects, enforced in-test.
//
// The C `.so`'s exported (`T`) dynamic symbols must all be exported by the Rust
// `.so` under the exact same name, and each must be resolvable through dlsym.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined (`T`/`D`/`B`/`W` with an address) dynamic symbols, per `nm -D`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>"; weak/undefined entries have no address.
        if f.len() == 3 && matches!(f[1], "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r") {
            set.insert(f[2].to_string());
        }
    }
    set
}

/// Symbols the toolchain adds around a shared object; not part of the API.
fn is_toolchain_artifact(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("_ZN")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || matches!(
            name,
            "__gmon_start__"
                | "_init"
                | "_fini"
                | "__bss_start"
                | "_edata"
                | "_end"
                | "gettid"
                | "statx"
        )
}

#[test]
fn phase_d_symbol_parity_is_empty_diff() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());

    let c_api: BTreeSet<&String> = c.iter().filter(|s| !is_toolchain_artifact(s)).collect();
    let missing: Vec<&&String> = c_api.iter().filter(|s| !r.contains(**s)).collect();

    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports: {c:?}\nRust exports: {r:?}",
        missing.len()
    );

    // Sanity: the four documented entry points really are in the C set, so the
    // assertion above cannot pass vacuously because nm produced nothing.
    for want in ["printLine", "bad", "good", "driver"] {
        assert!(
            c.contains(want),
            "expected `{want}` in the C .so's dynamic symbols; got {c:?}"
        );
        assert!(
            r.contains(want),
            "expected `{want}` in the Rust .so's dynamic symbols; got {r:?}"
        );
    }
    assert_eq!(c_api.len(), 4, "C public API surface changed: {c_api:?}");
}

#[test]
fn phase_d_statics_are_not_exported_by_either() {
    // `helperBad` and `helperGood1` are `static` in driver.c, so neither object
    // may expose them.
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    for hidden in ["helperBad", "helperGood1"] {
        assert!(!c.contains(hidden), "C unexpectedly exports {hidden}");
        assert!(!r.contains(hidden), "Rust unexpectedly exports {hidden}");
    }
}

#[test]
fn phase_d_every_symbol_resolves_via_dlsym() {
    // resolve() panics with the symbol name if any dlsym lookup fails; calling
    // it for both objects proves the exports are usable, not just listed.
    let _ = c_api();
    let _ = rust_api();
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("ldd")
        .arg("-r")
        .arg(rust_so_path())
        .output()
        .expect("run ldd -r");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved symbols:\n{}",
        bad.join("\n")
    );
}
