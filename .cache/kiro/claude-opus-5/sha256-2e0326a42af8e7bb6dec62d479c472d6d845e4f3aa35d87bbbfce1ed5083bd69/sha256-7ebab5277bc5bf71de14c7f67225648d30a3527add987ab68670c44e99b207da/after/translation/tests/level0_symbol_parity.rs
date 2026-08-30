//! Exported-symbol parity: every dynamic symbol the C `libdriver.so` defines
//! must also be defined by the Rust `libdriver.so` under the identical name.
//!
//! This is the `nm -D` comparison expressed as a test so it cannot drift.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::libs;

/// Names that belong to the toolchain/runtime rather than the translation, and
/// which therefore are not expected to appear on both sides.
fn is_runtime_symbol(name: &str) -> bool {
    name.starts_with("_")
        || name.starts_with("rust_")
        || name.starts_with("__")
        || matches!(
            name,
            "_init" | "_fini" | "rust_eh_personality" | "rust_begin_unwind"
        )
}

/// Dynamic symbols *defined* (not imported) by `so`, via `nm -D --defined-only`.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // Format: "<addr> <type> <name>"; keep globally visible code/data.
            let mut parts = line.split_whitespace();
            let _addr = parts.next()?;
            let kind = parts.next()?;
            let name = parts.next()?;
            // Uppercase type letters denote global/external linkage.
            if kind.chars().next()?.is_ascii_uppercase() {
                Some(name.to_string())
            } else {
                None
            }
        })
        .filter(|name| !is_runtime_symbol(name))
        .collect()
}

fn c_so() -> std::path::PathBuf {
    // Loading the libraries first ensures the C object has been built.
    let _ = libs();
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_so() -> std::path::PathBuf {
    let _ = libs();
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    for candidate in [
        deps.join("libdriver.so"),
        deps.parent().unwrap().join("libdriver.so"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("Rust cdylib not found");
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&c_so());
    let rust_syms = defined_dynamic_symbols(&rust_so());

    assert!(
        !c_syms.is_empty(),
        "sanity check: the C library should export at least one symbol"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust library is missing exports present in the C library: {missing:?}\n\
         C   : {c_syms:?}\n\
         Rust: {rust_syms:?}"
    );
}

/// The header's public entry point plus the helper the C file leaves with
/// external linkage must both be resolvable through `dlsym` in both libraries.
#[test]
fn documented_entry_points_are_loadable() {
    for name in ["driver", "printHexCharLine"] {
        // Panics with a clear message if either library lacks the symbol.
        let _ = common::char_fns(name);
    }
}
