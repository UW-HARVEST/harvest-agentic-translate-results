//! Phase D — symbol parity, enforced by the test suite itself so that a
//! regression in the `#[no_mangle]` export surface fails `cargo test`.

mod common;

use common::{c_lib_path, rust_lib_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of all globally-defined dynamic symbols in `so`.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run `nm` (binutils required)");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm").args(["-D", "-u"]).arg(so).output().expect("run `nm`");
    assert!(out.status.success(), "nm -u failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// EVERY symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name.
#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_lib_path());
    let r = defined_symbols(&rust_lib_path());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports:    {c:?}\nRust exports: {r:?}"
    );
    // Guard against a vacuous pass (e.g. `nm` returning nothing at all).
    assert!(
        c.contains("merge_sort"),
        "expected `merge_sort` among the C exports, got {c:?}"
    );
    assert!(
        r.contains("merge_sort"),
        "expected `merge_sort` among the Rust exports, got {r:?}"
    );
}

/// The Rust `.so` must not need any non-libc symbol resolved at load time.
/// (`dlopen` in the harness would already fail, but this pins the reason.)
#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let undef = undefined_symbols(&rust_lib_path());
    let leftovers: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("__cxa_")
                || s.as_str() == "__gmon_start__"
                || s.as_str() == "gettid"
                || s.as_str() == "statx")
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has undefined non-libc symbols: {leftovers:?}"
    );
}

/// The C `static` helpers must remain absent from BOTH `.so`s — exporting them
/// from Rust would make the two ABIs differ in the other direction.
#[test]
fn static_helpers_are_exported_by_neither() {
    let c = defined_symbols(&c_lib_path());
    let r = defined_symbols(&rust_lib_path());
    for name in [
        "spritebatch_internal_sprite_less_than_or_equal",
        "spritebatch_internal_merge_sort_iteration",
        "spritebatch_internal_merge_sort_recurse",
    ] {
        assert!(!c.contains(name), "C unexpectedly exports `{name}`");
        assert!(
            !r.contains(name),
            "Rust exports `{name}`, but it is `static` in the C and must not be \
             part of the ABI"
        );
    }
}
