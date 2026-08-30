//! Phase D — symbol parity, enforced as a test so it cannot drift.

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
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next_back()?;
            let kind = it.next_back()?;
            // Global/weak code & data only; skip local (lowercase) entries.
            if kind.len() == 1 && kind.chars().next().unwrap().is_ascii_uppercase() {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());

    assert!(c.contains("driver"), "C .so must export `driver`, got {c:?}");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
}

#[test]
fn d2_rust_exports_no_undeclared_extras() {
    // Not a hard requirement of the task, but it catches accidentally-exported
    // helpers that would widen the public ABI beyond the C library's.
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(extra.is_empty(), "Rust .so exports extra public symbols: {extra:?}");
}

#[test]
fn d3_rust_so_has_no_unresolvable_non_libc_symbols() {
    let out = Command::new("nm").args(["-D", "-u"]).arg(rust_so_path()).output().expect("nm");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    // Everything the Rust object imports must be resolvable at load time; the
    // successful dlopen in `impls()` is the real proof.
    let _ = impls();
    for sym in &undef {
        assert!(
            !sym.contains("unimplemented") && !sym.contains("todo"),
            "suspicious unresolved symbol {sym}"
        );
    }
}

#[test]
fn d4_no_stubs_in_rust_source() {
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    for bad in ["unimplemented!", "todo!", "unreachable!(\"stub", "panic!(\"stub"] {
        assert!(!src.contains(bad), "src/lib.rs contains a stub marker: {bad}");
    }
}

/// Harness integrity: prove the two function pointers really come from two
/// different shared objects. Without this, a path mix-up could silently compare
/// the C library against itself and make every differential test vacuous.
#[test]
fn d5_harness_loads_two_distinct_objects() {
    let f = impls();
    assert_ne!(
        f.c as usize, f.rust as usize,
        "C and Rust `driver` resolved to the SAME address — the harness is comparing one \
         library against itself"
    );

    let origin = |p: usize| -> String {
        let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
        let ok = unsafe { libc::dladdr(p as *const libc::c_void, &mut info) };
        assert!(ok != 0, "dladdr failed");
        unsafe { std::ffi::CStr::from_ptr(info.dli_fname) }
            .to_string_lossy()
            .into_owned()
    };

    let c_from = origin(f.c as usize);
    let r_from = origin(f.rust as usize);
    assert_ne!(c_from, r_from, "both symbols came from {c_from}");
    assert!(c_from.contains("c_src"), "C `driver` came from {c_from}, expected c_src/build");
    assert!(
        r_from.contains("target"),
        "Rust `driver` came from {r_from}, expected the cargo target dir"
    );
    eprintln!("C   driver <- {c_from}");
    eprintln!("Rust driver <- {r_from}");
}
