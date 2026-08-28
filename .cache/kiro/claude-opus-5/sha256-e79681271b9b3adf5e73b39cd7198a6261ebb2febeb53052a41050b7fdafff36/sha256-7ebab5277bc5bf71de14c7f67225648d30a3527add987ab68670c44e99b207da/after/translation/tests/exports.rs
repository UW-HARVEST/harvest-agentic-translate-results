//! Verifies that the Rust cdylib exports every dynamic symbol the C shared
//! library exports, under the same name, and that each one is resolvable via
//! `dlsym`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Global data/code definitions only; skip weak/local/debug entries.
            matches!(kind, "T" | "D" | "B" | "R").then(|| name.to_string())
        })
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    let c = exported_symbols(&common::c_so_path_pub());
    let rs = exported_symbols(&common::rust_so_path_pub());

    assert!(
        !c.is_empty(),
        "no symbols parsed from the C library; the nm parsing is broken"
    );

    let missing: Vec<&String> = c.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing exports present in the C .so: {missing:?}"
    );

    // The full public API from include/lib.h plus the internal helpers that the
    // C translation unit leaves with external linkage.
    for expected in [
        "buffapp",
        "create_buffer",
        "append_to_buffer",
        "destroy_buffer",
        "get_operation_name",
        "perform_operation",
    ] {
        assert!(c.contains(expected), "C .so unexpectedly lacks {expected}");
        assert!(rs.contains(expected), "Rust .so lacks {expected}");
    }
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_rust() {
    let c = exported_symbols(&common::c_so_path_pub());
    let rust_path = common::rust_so_path_pub();

    // SAFETY: loading a library built from this repo and only taking addresses.
    unsafe {
        let lib = libloading::Library::new(&rust_path).expect("dlopen Rust .so");
        for name in &c {
            let mut key = name.clone().into_bytes();
            key.push(0);
            let sym: Result<libloading::Symbol<*const ()>, _> = lib.get(&key);
            assert!(sym.is_ok(), "dlsym({name}) failed on the Rust .so");
        }
    }
}
