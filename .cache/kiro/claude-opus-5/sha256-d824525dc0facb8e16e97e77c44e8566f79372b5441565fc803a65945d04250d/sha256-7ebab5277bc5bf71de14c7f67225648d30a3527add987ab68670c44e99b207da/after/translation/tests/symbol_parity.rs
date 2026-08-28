//! Exported-symbol parity: every dynamic symbol the C shared library defines
//! must also be defined by the Rust shared library, under the exact same name
//! and with the same symbol type.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only <so>` -> { symbol name: symbol type }
fn defined_dynamic_symbols(so: &Path) -> BTreeMap<String, char> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}:\n{}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        // "<value> <type> <name>", value may be blank for undefined entries.
        let mut fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 2 {
            continue;
        }
        let name = fields.pop().unwrap().to_string();
        let ty = fields.pop().unwrap();
        let Some(ty) = ty.chars().next().filter(|_| ty.len() == 1) else {
            continue;
        };
        map.insert(name, ty);
    }
    assert!(!map.is_empty(), "nm reported no symbols for {}", so.display());
    map
}

/// Symbols that belong to the Rust/`cdylib` runtime rather than to the
/// translated source. The Rust library is allowed to export extra symbols; it
/// is only forbidden to be *missing* any that C exports.
#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let rust_syms = defined_dynamic_symbols(&rust_so_path());

    let mut problems = Vec::new();
    for (name, c_ty) in &c_syms {
        match rust_syms.get(name) {
            None => problems.push(format!("missing from the Rust .so: `{name}` ({c_ty})")),
            Some(rust_ty) if rust_ty != c_ty => problems.push(format!(
                "`{name}` has type {rust_ty} in the Rust .so but {c_ty} in the C .so"
            )),
            Some(_) => {}
        }
    }

    assert!(
        problems.is_empty(),
        "exported-symbol mismatch between {} and {}:\n  {}",
        c_so_path().display(),
        rust_so_path().display(),
        problems.join("\n  ")
    );
}

/// The symbols must additionally be reachable through `dlsym` with the exact
/// same name, which is what an external caller actually relies on.
#[test]
fn every_c_symbol_is_resolvable_in_the_rust_so() {
    let mut missing = Vec::new();
    for (name, ty) in defined_dynamic_symbols(&c_so_path()) {
        // Only code symbols are callable entry points.
        if ty != 'T' && ty != 'W' && ty != 'i' {
            continue;
        }
        let mut with_nul = name.clone().into_bytes();
        with_nul.push(0);
        if unsafe { rust_lib().get::<*mut c_void>(&with_nul) }.is_err() {
            missing.push(name);
        }
    }
    assert!(missing.is_empty(), "not dlsym-able in the Rust .so: {missing:?}");
}

/// Guards against a regression in the other direction for the documented API:
/// the three functions the C source defines with external linkage.
#[test]
fn documented_api_is_present_in_both() {
    for name in ["driver", "forward_goto_example", "open_with_cleanup"] {
        for (which, lib) in [("C", c_lib()), ("Rust", rust_lib())] {
            let mut with_nul = name.as_bytes().to_vec();
            with_nul.push(0);
            assert!(
                unsafe { lib.get::<*mut c_void>(&with_nul) }.is_ok(),
                "{which} .so does not export `{name}`"
            );
        }
    }
}
