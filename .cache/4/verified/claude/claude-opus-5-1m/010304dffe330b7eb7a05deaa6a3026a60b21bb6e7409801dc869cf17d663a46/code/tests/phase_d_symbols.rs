//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every symbol the C shared object defines dynamically must also be defined by
//! the Rust shared object under the exact same name, and it must actually be
//! callable through `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {} failed: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn phase_d_symbol_diff_is_empty() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());

    assert!(
        c.contains("driver") && c.contains("main"),
        "unexpected C export set: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   : {c:?}\n\
         Rust: {r:?}"
    );
}

#[test]
fn phase_d_every_c_symbol_is_dlsym_able_in_rust() {
    for sym in defined_dynamic_symbols(&c_so_path()) {
        assert!(
            rust_impl().has_symbol(&sym),
            "dlsym(\"{sym}\") failed on the Rust .so although the C .so exports it"
        );
    }
}

#[test]
fn phase_d_rust_so_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let unresolved: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            // everything the Rust std imports comes from libc / libgcc's unwinder
            !s.contains("@GLIBC")
                && !s.contains("@GCC")
                && !s.starts_with("_ITM_")
                && !s.starts_with("_Unwind_")
                && *s != "__gmon_start__"
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has undefined non-libc symbols: {unresolved:?}"
    );
}

/// The Rust `.so` must not leak extra C-ABI entry points that the C `.so` does
/// not have (a stub/extra symbol would be a translation artefact).
#[test]
fn phase_d_rust_exports_nothing_extra() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
}
